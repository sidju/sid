//! Core C-FFI types: [`CType`], [`CFuncSig`], [`CFunc`].

use std::sync::Arc;

use libloading::Library;

use crate::type_system::SidType;

// ── C type mapping ────────────────────────────────────────────────────────────

/// A subset of C types that can be bridged to [`DataValue`] automatically.
#[derive(Debug, Clone, PartialEq)]
pub enum CType {
    Void,
    Int,     // C `int`    → DataValue::Int (i64)
    Long,    // C `long`   → DataValue::Int (i64)
    SizeT,   // C `size_t` → DataValue::Int (i64)
    Float,   // C `float`  → DataValue::Float (f64)
    Double,  // C `double` → DataValue::Float (f64)
    CString, // C `char *` → DataValue::Str
    /// Represents a C pointer type.  Carries the SID pointee type for display;
    /// at the ABI level all pointers are the same width.
    /// `SidType::Any` is used when the pointee type is `void` or unknown.
    Pointer(SidType),
}

impl CType {
    /// Map this [`CType`] to the corresponding libffi [`libffi::middle::Type`].
    pub fn to_ffi_type(&self) -> libffi::middle::Type {
        use libffi::middle::Type;
        match self {
            CType::Void => Type::void(),
            CType::Int => Type::i32(),
            CType::Long | CType::SizeT => Type::i64(),
            CType::Float => Type::f32(),
            CType::Double => Type::f64(),
            CType::CString | CType::Pointer(_) => Type::pointer(),
        }
    }
}

// ── Parsed function signature ─────────────────────────────────────────────────

/// A parsed C function signature: return type, name, ordered parameter types,
/// and the library that provides it.
///
/// `lib_name` is always populated at header-load time (by `c_load_header`) so
/// `call_cfuncsig` can resolve the symbol without any Option checks.
pub struct CFuncSig {
    pub name: String,
    pub ret: CType,
    /// Parameter types in declaration order.  Unnamed parameters are fine.
    pub params: Vec<CType>,
    /// Name under which the providing library is registered in
    /// [`GlobalState::libraries`].  Must be pre-loaded with `c_link_lib`
    /// before any function with this signature is called.
    pub lib_name: String,
}

impl std::fmt::Debug for CFuncSig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CFuncSig")
            .field("name", &self.name)
            .field("ret",  &self.ret)
            .field("params", &self.params)
            .field("lib_name", &self.lib_name)
            .finish()
    }
}
impl Clone for CFuncSig {
    fn clone(&self) -> Self {
        CFuncSig {
            name:     self.name.clone(),
            ret:      self.ret.clone(),
            params:   self.params.clone(),
            lib_name: self.lib_name.clone(),
        }
    }
}
/// Equality ignores `lib_name` — two signatures with the same name, return
/// type, and parameter types are considered equal regardless of which library
/// provides them.
impl PartialEq for CFuncSig {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.ret == other.ret
            && self.params == other.params
    }
}

// ── Loaded C function ─────────────────────────────────────────────────────────

/// A newtype around a raw function pointer that is [`Send`] + [`Sync`].
///
/// # Safety
/// We assume function pointers from dynamically-linked libraries are safe to
/// call from any thread (the C ABI contract).
#[derive(Copy, Clone)]
pub(super) struct FnPtr(pub(super) *const ());
unsafe impl Send for FnPtr {}
unsafe impl Sync for FnPtr {}
impl std::fmt::Debug for FnPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:p}", self.0)
    }
}
impl PartialEq for FnPtr {
    fn eq(&self, other: &Self) -> bool { self.0 == other.0 }
}

/// A dynamically-loaded C function that can be called via libffi.
pub struct CFunc {
    /// Keep the library alive for as long as this handle exists.
    pub _lib: Arc<Library>,
    pub name: String,
    pub(super) fn_ptr: FnPtr,
    pub sig: CFuncSig,
}

impl std::fmt::Debug for CFunc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CFunc({})", self.name)
    }
}
impl PartialEq for CFunc {
    /// Two [`CFunc`]s are equal iff they point to the same function.
    fn eq(&self, other: &Self) -> bool { self.fn_ptr == other.fn_ptr }
}
impl Clone for CFunc {
    fn clone(&self) -> Self {
        CFunc {
            _lib: Arc::clone(&self._lib),
            name: self.name.clone(),
            fn_ptr: self.fn_ptr,
            sig: self.sig.clone(),
        }
    }
}
