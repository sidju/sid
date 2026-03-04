//! C foreign-function-interface helpers.
//!
//! This module provides:
//! - [`CType`]      — a small enum covering the C primitive types we can bridge
//! - [`CFuncSig`]   — a parsed C function signature (name + param/return types)
//! - [`CFunc`]      — a loaded, callable C function (library handle + pointer + sig)
//! - [`parse_c_header`] — parse a C header via the system C preprocessor + lang-c
//! - [`load_c_functions`] — load a shared library and resolve the supplied symbols
//! - [`call_c_function`]  — call a [`CFunc`] from a [`DataValue`] argument

use std::ffi::{CString, c_char};
use std::sync::Arc;

use anyhow::{bail, Result};
use libloading::Library;

use crate::DataValue;
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

/// A parsed C function signature: return type, name, and ordered parameter types.
///
/// After `c_link_lib` is called the `lib` field holds the shared library that
/// provides this function.  It is `None` while the signature is still
/// unlinked (i.e. produced by `c_load_header` but not yet linked).
pub struct CFuncSig {
    pub name: String,
    pub ret: CType,
    /// Parameter types in declaration order.  Unnamed parameters are fine.
    pub params: Vec<CType>,
    /// Shared-library handle set by `c_link_lib`.  `None` = not yet linked.
    pub lib: Option<Arc<Library>>,
}

impl std::fmt::Debug for CFuncSig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CFuncSig")
            .field("name", &self.name)
            .field("ret",  &self.ret)
            .field("params", &self.params)
            .field("linked", &self.lib.is_some())
            .finish()
    }
}
impl Clone for CFuncSig {
    fn clone(&self) -> Self {
        CFuncSig {
            name:   self.name.clone(),
            ret:    self.ret.clone(),
            params: self.params.clone(),
            lib:    self.lib.clone(),
        }
    }
}
/// Equality ignores the library handle — two signatures with the same name,
/// return type, and parameter types are considered equal regardless of which
/// library (if any) they are linked against.
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
struct FnPtr(*const ());
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
    fn_ptr: FnPtr,
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

// ── C header parsing via system preprocessor + lang-c ────────────────────────

/// Parse a C header file and return all bridgeable function signatures.
///
/// The file is processed through the system C preprocessor (`gcc -E` on Linux,
/// `clang -E` on macOS) so that `#include` guards, macros, and transitive
/// includes are fully resolved before parsing.  Variadic functions, function
/// pointers, struct/union/enum/typedef declarations, and any declaration whose
/// types cannot be bridged are silently skipped.
pub fn parse_c_header(path: &str) -> Result<Vec<CFuncSig>> {
    let config = lang_c::driver::Config::default();
    let parse = lang_c::driver::parse(&config, path)
        .map_err(|e| anyhow::anyhow!("failed to parse '{}': {}", path, e))?;
    Ok(extract_function_sigs(&parse.unit))
}

/// Walk a fully preprocessed translation unit and return bridgeable function
/// signatures.
fn extract_function_sigs(unit: &lang_c::ast::TranslationUnit) -> Vec<CFuncSig> {
    unit.0.iter().filter_map(|ext| {
        if let lang_c::ast::ExternalDeclaration::Declaration(decl) = &ext.node {
            try_extract_func_sig(&decl.node)
        } else {
            None
        }
    }).collect()
}

/// Attempt to extract a bridgeable function signature from a top-level
/// declaration.  Returns `None` for anything that is not a simple function.
fn try_extract_func_sig(decl: &lang_c::ast::Declaration) -> Option<CFuncSig> {
    use lang_c::ast::*;

    // Work with the first (and usually only) declarator in the declaration.
    let init_decl = decl.declarators.first()?;
    let declarator = &init_decl.node.declarator.node;

    // Only plain identifiers are function names we can bridge; skip nested
    // declarators like pointer-to-function `(*fn_ptr)(…)`.
    let name = match &declarator.kind.node {
        DeclaratorKind::Identifier(id) => id.node.name.clone(),
        _ => return None,
    };

    // The innermost derived declarator must be a function call; otherwise this
    // is a variable, array, or pointer declaration.
    let func_decl = match declarator.derived.first() {
        Some(d) => match &d.node {
            DerivedDeclarator::Function(f) => &f.node,
            _ => return None,
        },
        None => return None,
    };

    // Skip variadic functions — we cannot bridge them safely.
    if func_decl.ellipsis == Ellipsis::Some {
        return None;
    }

    // Any Pointer derived declarators that appear *after* (outer than) the
    // Function entry indicate a pointer return type.
    let has_return_ptr = declarator.derived[1..].iter().any(|d| {
        matches!(&d.node, DerivedDeclarator::Pointer(_))
    });

    let ret = specifiers_to_ctype(&decl.specifiers, has_return_ptr)?;
    let params = extract_params(&func_decl.parameters)?;

    Some(CFuncSig { name, ret, params, lib: None })
}

/// Extract the parameter types from a function's parameter list.
///
/// Returns `None` if any parameter type cannot be bridged.
fn extract_params(
    params: &[lang_c::span::Node<lang_c::ast::ParameterDeclaration>],
) -> Option<Vec<CType>> {
    use lang_c::ast::*;

    if params.is_empty() {
        return Some(vec![]);
    }

    // A single `void` parameter means the function takes no arguments.
    if params.len() == 1 {
        let p = &params[0].node;
        let is_void_only = p.declarator.is_none()
            && p.specifiers.iter().any(|s| {
                matches!(
                    &s.node,
                    DeclarationSpecifier::TypeSpecifier(ts)
                    if matches!(ts.node, TypeSpecifier::Void)
                )
            });
        if is_void_only {
            return Some(vec![]);
        }
    }

    let mut result = Vec::new();
    for param_node in params {
        let param = &param_node.node;

        // Is there a pointer in the parameter's declarator?
        let has_ptr = param.declarator.as_ref().map(|d| {
            d.node.derived.iter().any(|der| {
                matches!(&der.node, DerivedDeclarator::Pointer(_))
            })
        }).unwrap_or(false);

        let ctype = specifiers_to_ctype(&param.specifiers, has_ptr)?;
        if ctype != CType::Void {
            result.push(ctype);
        }
    }
    Some(result)
}

/// Map a set of C declaration specifiers (plus a pointer flag) to a [`CType`].
///
/// Returns `None` if the type combination cannot be bridged.
fn specifiers_to_ctype(
    specs: &[lang_c::span::Node<lang_c::ast::DeclarationSpecifier>],
    has_ptr: bool,
) -> Option<CType> {
    use lang_c::ast::{DeclarationSpecifier, TypeSpecifier};

    // Collect just the TypeSpecifier variants; ignore qualifiers, storage class, etc.
    let type_specs: Vec<&TypeSpecifier> = specs.iter().filter_map(|s| {
        if let DeclarationSpecifier::TypeSpecifier(ts) = &s.node {
            Some(&ts.node)
        } else {
            None
        }
    }).collect();

    let has_char   = type_specs.iter().any(|s| matches!(s, TypeSpecifier::Char));
    let has_void   = type_specs.iter().any(|s| matches!(s, TypeSpecifier::Void));
    let has_float  = type_specs.iter().any(|s| matches!(s, TypeSpecifier::Float));
    let has_double = type_specs.iter().any(|s| matches!(s, TypeSpecifier::Double));
    let has_short  = type_specs.iter().any(|s| matches!(s, TypeSpecifier::Short));
    let has_int    = type_specs.iter().any(|s| matches!(s, TypeSpecifier::Int));
    let long_count = type_specs.iter().filter(|s| matches!(s, TypeSpecifier::Long)).count();

    // `char *` → CString; bare `char` → Int.
    if has_char {
        return if has_ptr { Some(CType::CString) } else { Some(CType::Int) };
    }

    // Any other pointer — determine the pointee SidType for display.
    if has_ptr {
        let pointee = if has_void {
            SidType::Any
        } else if has_double || has_float {
            SidType::Float
        } else if has_int || has_short || long_count > 0 {
            SidType::Int
        } else {
            // Typedef or unknown base type — fall back to Any
            typedef_name(&type_specs)
                .and_then(sid_type_for_typedef)
                .unwrap_or(SidType::Any)
        };
        return Some(CType::Pointer(pointee));
    }

    // Non-pointer primitive types.
    if has_void   { return Some(CType::Void); }
    if has_double { return Some(CType::Double); }
    if has_float  { return Some(CType::Float); }
    if long_count >= 2                     { return Some(CType::Long); } // long long
    if long_count == 1 && !has_int         { return Some(CType::Long); } // bare long
    if has_int || has_short || long_count > 0 { return Some(CType::Int); }

    // Typedef names: size_t, int32_t, etc.
    if let Some(name) = typedef_name(&type_specs) {
        return match name {
            "size_t" => Some(CType::SizeT),
            "ssize_t" | "ptrdiff_t" | "intmax_t" | "uintmax_t"
            | "intptr_t" | "uintptr_t" => Some(CType::Long),
            "uint8_t" | "uint16_t" | "uint32_t" | "uint64_t"
            | "int8_t"  | "int16_t"  | "int32_t"  | "int64_t"
            | "off_t" | "pid_t" | "uid_t" | "gid_t" | "mode_t"
            | "dev_t" | "ino_t" | "nlink_t" | "socklen_t" => Some(CType::Int),
            _ => None,
        };
    }

    None // unknown type — caller skips this function
}

/// Extract the typedef name from a list of type specifiers, if present.
fn typedef_name<'a>(specs: &[&'a lang_c::ast::TypeSpecifier]) -> Option<&'a str> {
    specs.iter().find_map(|s| {
        if let lang_c::ast::TypeSpecifier::TypedefName(name) = s {
            Some(name.node.name.as_str())
        } else {
            None
        }
    })
}

/// Map a known C typedef name to its SID pointee type.
fn sid_type_for_typedef(name: &str) -> Option<SidType> {
    match name {
        "size_t" | "uint8_t" | "uint16_t" | "uint32_t" | "uint64_t"
        | "int8_t" | "int16_t" | "int32_t" | "int64_t"
        | "ssize_t" | "ptrdiff_t" | "intptr_t" | "uintptr_t"
        | "off_t" | "pid_t" | "uid_t" | "gid_t" => Some(SidType::Int),
        _ => None,
    }
}

// ── Dynamic library loading ───────────────────────────────────────────────────

/// Open a shared library and associate it with every signature whose name is
/// exported by that library.  Unresolved signatures are left with `lib: None`.
///
/// Returns a new `Vec<CFuncSig>` with the matching entries updated.
///
/// # Safety
/// Loading native libraries is inherently unsafe.
pub fn link_sigs_to_lib(lib_path: &str, sigs: &[CFuncSig]) -> Result<Vec<CFuncSig>> {
    // SAFETY: opening a shared library.
    let lib = unsafe { Library::new(lib_path) }
        .map_err(|e| anyhow::anyhow!("failed to load '{}': {}", lib_path, e))?;
    let lib = Arc::new(lib);

    let mut out: Vec<CFuncSig> = sigs.to_vec();
    for sig in &mut out {
        if sig.lib.is_some() {
            continue; // already linked by a previous c_link_lib call
        }
        let sym_name = CString::new(sig.name.as_str()).unwrap();
        // SAFETY: we only read whether the symbol exists; we do not call it.
        let found = unsafe {
            lib.get::<unsafe extern "C" fn()>(sym_name.as_bytes_with_nul()).is_ok()
        };
        if found {
            sig.lib = Some(Arc::clone(&lib));
        }
    }
    Ok(out)
}

// ── Calling C functions ───────────────────────────────────────────────────────

/// Call `func` (a pre-loaded [`CFunc`]) with the given `arg`.
///
/// - 0-param functions: pass `None`.
/// - 1-param functions: pass the single [`DataValue`].
/// - N-param functions: pass `DataValue::List` with items in declaration order.
///
/// # Safety
/// Calls arbitrary C code.  The caller must supply arguments matching the
/// declared C types.
pub fn call_c_function(func: &CFunc, arg: Option<DataValue>) -> Result<Option<DataValue>> {
    call_fn_ptr(func.fn_ptr.0, &func.sig, arg)
}

/// Call a [`CFuncSig`] that has been linked via `c_link_lib`.
///
/// The symbol is looked up by name in the stored library at each call — no
/// function pointer is cached.
///
/// Returns an error if the signature has not been linked yet.
///
/// # Safety
/// Calls arbitrary C code.  The caller must supply arguments matching the
/// declared C types.
pub fn call_cfuncsig(sig: &CFuncSig, arg: Option<DataValue>) -> Result<Option<DataValue>> {
    let lib = sig.lib.as_ref().ok_or_else(|| anyhow::anyhow!(
        "'{}' is an unlinked C function signature — call c_link_lib first",
        sig.name
    ))?;
    let sym_name = CString::new(sig.name.as_str()).unwrap();
    // SAFETY: we read the raw function pointer; it is not called until
    // call_fn_ptr, which builds a correct CIF from the signature.
    let fn_ptr: *const () = unsafe {
        match lib.get::<unsafe extern "C" fn()>(sym_name.as_bytes_with_nul()) {
            Ok(sym) => *sym as *const (),
            Err(e) => bail!("'{}': symbol not found in linked library: {}", sig.name, e),
        }
    };
    call_fn_ptr(fn_ptr, sig, arg)
}

/// Core call implementation: marshal `arg` according to `sig`, invoke via
/// libffi, and unmarshal the return value.
fn call_fn_ptr(
    fn_ptr: *const (),
    sig: &CFuncSig,
    arg: Option<DataValue>,
) -> Result<Option<DataValue>> {
    use libffi::middle::{Cif, CodePtr};

    let params = &sig.params;

    // Collect DataValue arguments.
    let arg_values: Vec<DataValue> = match (params.len(), arg) {
        (0, _) => vec![],
        (1, Some(v)) => vec![v],
        (n, Some(DataValue::List(items))) if items.len() == n => items,
        (n, _) => bail!(
            "'{}': expected {} argument(s)",
            sig.name, n
        ),
    };

    // Marshal each Rust value into a C-compatible form that lives long enough
    // for the libffi call.
    enum StoredArg {
        I32(i32),
        I64(i64),
        F32(f32),
        F64(f64),
        /// The CString keeps the allocation alive; `ptr` is the raw char pointer we
        /// hand to libffi (as `*const c_char`).
        CStr(#[allow(dead_code)] CString, *const c_char),
        Ptr(*const std::ffi::c_void),
    }

    let mut stored: Vec<StoredArg> = Vec::with_capacity(arg_values.len());
    for (val, ctype) in arg_values.iter().zip(params.iter()) {
        let s = match (val, ctype) {
            (DataValue::Int(n), CType::Int) => StoredArg::I32(*n as i32),
            (DataValue::Int(n), CType::Long | CType::SizeT) => StoredArg::I64(*n),
            (DataValue::Float(f), CType::Float) => StoredArg::F32(*f as f32),
            (DataValue::Float(f), CType::Double) => StoredArg::F64(*f),
            (DataValue::Str(s), CType::CString) => {
                let cs = CString::new(s.as_str()).map_err(|_| anyhow::anyhow!(
                    "'{}': string argument contains interior NUL byte",
                    sig.name
                ))?;
                let ptr = cs.as_ptr();
                StoredArg::CStr(cs, ptr)
            }
            // Accept both a raw integer address and a typed Pointer value.
            (DataValue::Int(n), CType::Pointer(_)) => {
                StoredArg::Ptr(*n as usize as *const std::ffi::c_void)
            }
            (DataValue::Pointer { addr, .. }, CType::Pointer(_)) => {
                StoredArg::Ptr(*addr as *const std::ffi::c_void)
            }
            _ => bail!(
                "'{}': argument type mismatch (value {:?} vs expected C type {:?})",
                sig.name, val, ctype
            ),
        };
        stored.push(s);
    }

    // Build libffi argument list (holds references into `stored`).
    // Using a `for` loop (rather than `.map()`) avoids creating local variables
    // that outlive their scope — `stored[i]` lives for the whole function.
    let ffi_arg_types: Vec<libffi::middle::Type> =
        params.iter().map(CType::to_ffi_type).collect();

    let mut ffi_args: Vec<libffi::middle::Arg> = Vec::with_capacity(stored.len());
    for s in &stored {
        let a = match s {
            StoredArg::I32(v) => libffi::middle::arg(v),
            StoredArg::I64(v) => libffi::middle::arg(v),
            StoredArg::F32(v) => libffi::middle::arg(v),
            StoredArg::F64(v) => libffi::middle::arg(v),
            // Pass `&ptr` so that libffi reads the char* value from the stack slot.
            StoredArg::CStr(_, ptr) => libffi::middle::arg(ptr),
            StoredArg::Ptr(ptr) => libffi::middle::arg(ptr),
        };
        ffi_args.push(a);
    }

    let cif = Cif::new(ffi_arg_types, sig.ret.to_ffi_type());
    let code_ptr = CodePtr(fn_ptr as *mut _);

    // SAFETY: We built the CIF to match the declared C signature.
    let result = match &sig.ret {
        CType::Void => {
            unsafe { cif.call::<()>(code_ptr, &ffi_args) };
            None
        }
        CType::Int => {
            let v: i32 = unsafe { cif.call(code_ptr, &ffi_args) };
            Some(DataValue::Int(v as i64))
        }
        CType::Long | CType::SizeT => {
            let v: i64 = unsafe { cif.call(code_ptr, &ffi_args) };
            Some(DataValue::Int(v))
        }
        CType::Float => {
            let v: f32 = unsafe { cif.call(code_ptr, &ffi_args) };
            Some(DataValue::Float(v as f64))
        }
        CType::Double => {
            let v: f64 = unsafe { cif.call(code_ptr, &ffi_args) };
            Some(DataValue::Float(v))
        }
        CType::CString => {
            let ptr: *const c_char = unsafe { cif.call(code_ptr, &ffi_args) };
            if ptr.is_null() {
                None
            } else {
                let s = unsafe { std::ffi::CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned();
                Some(DataValue::Str(s))
            }
        }
        CType::Pointer(pointee_ty) => {
            let ptr: *mut std::ffi::c_void = unsafe { cif.call(code_ptr, &ffi_args) };
            if ptr.is_null() {
                None
            } else {
                Some(DataValue::Pointer {
                    addr: ptr as usize,
                    pointee_ty: pointee_ty.clone(),
                })
            }
        }
    };

    Ok(result)
}
