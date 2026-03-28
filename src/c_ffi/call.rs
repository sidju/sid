//! Dynamic library loading and C function invocation via libffi.

use std::ffi::{CString, c_char};
use std::sync::Arc;

use anyhow::{bail, Result};
use libloading::Library;

use crate::DataValue;
use crate::type_system::SidType;
use super::types::{CFunc, CFuncSig, CType};

// ── Dynamic library loading ───────────────────────────────────────────────────

/// Open a shared library and return it wrapped in an [`Arc`].
///
/// # Safety
/// Loading native libraries is inherently unsafe.
pub fn open_library(lib_path: &str) -> Result<Arc<Library>> {
    // SAFETY: opening a shared library.
    let lib = unsafe { Library::new(lib_path) }
        .map_err(|e| anyhow::anyhow!("failed to load '{}': {}", lib_path, e))?;
    Ok(Arc::new(lib))
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

/// Call a [`CFuncSig`] by looking up the symbol in the pre-loaded library
/// registry.
///
/// The library must already be present in `libraries` (loaded via `c_link_lib`).
/// If it is not, this is a **fatal error** — unlike `call_c_function`, no lazy
/// loading is performed here.
///
/// # Safety
/// Calls arbitrary C code.  The caller must supply arguments matching the
/// declared C types.
pub fn call_cfuncsig(
    sig: &CFuncSig,
    arg: Option<DataValue>,
    libraries: &std::collections::HashMap<String, Arc<Library>>,
) -> Result<Option<DataValue>> {
    let lib = libraries.get(sig.lib_name.as_str())
        .ok_or_else(|| anyhow::anyhow!(
            "'{}': library '{}' is not loaded — call c_link_lib first",
            sig.name, sig.lib_name
        ))?;

    let sym_name = CString::new(sig.name.as_str()).unwrap();
    // SAFETY: we read the raw function pointer; it is not called until
    // call_fn_ptr, which builds a correct CIF from the signature.
    let fn_ptr: *const () = unsafe {
        match lib.get::<unsafe extern "C" fn()>(sym_name.as_bytes_with_nul()) {
            Ok(sym) => *sym as *const (),
            Err(e) => bail!("'{}': symbol not found in '{}': {}", sig.name, sig.lib_name, e),
        }
    };
    call_fn_ptr(fn_ptr, sig, arg)
}

/// Infer the C type to use for a variadic argument based on its runtime value.
///
/// Follows C default argument promotions:
/// - integers → `long` (i64 in a 64-bit register; `%d` reads the lower half)
/// - floats → `double` (`float` is promoted to `double` in variadic calls)
/// - strings / pointers → pointer
fn ctype_for_variadic(val: &DataValue) -> Result<CType> {
    match val {
        DataValue::Int(_)          => Ok(CType::Long),
        DataValue::Float(_)        => Ok(CType::Double),
        DataValue::Str(_)          => Ok(CType::CString),
        DataValue::Pointer { .. }  => Ok(CType::Pointer(SidType::Any)),
        other => bail!(
            "cannot infer C type for variadic argument: {:?}", other
        ),
    }
}

/// libffi, and unmarshal the return value.
fn call_fn_ptr(
    fn_ptr: *const (),
    sig: &CFuncSig,
    arg: Option<DataValue>,
) -> Result<Option<DataValue>> {
    use libffi::middle::{Cif, CodePtr};

    let params = &sig.params;

    // Collect DataValue arguments.
    // Variadic functions always expect a List (fixed + variadic args together).
    // Non-variadic functions use the existing 0/1/N convention.
    let arg_values: Vec<DataValue> = if sig.variadic {
        match arg {
            Some(DataValue::List(items)) if items.len() >= params.len() => items,
            Some(DataValue::List(items)) => bail!(
                "'{}': variadic call needs at least {} argument(s), got {}",
                sig.name, params.len(), items.len()
            ),
            _ => bail!(
                "'{}': variadic call expects a List of arguments", sig.name
            ),
        }
    } else {
        match (params.len(), arg) {
            (0, _) => vec![],
            (1, Some(v)) => vec![v],
            (n, Some(DataValue::List(items))) if items.len() == n => items,
            (n, _) => bail!(
                "'{}': expected {} argument(s)",
                sig.name, n
            ),
        }
    };

    // Build the full type list: declared types for fixed params, then inferred
    // types for any variadic arguments.
    let mut all_ctypes: Vec<CType> = params.clone();
    for val in &arg_values[params.len()..] {
        all_ctypes.push(ctype_for_variadic(val)?);
    }

    // Marshal each Rust value into a C-compatible form that lives long enough
    // for the libffi call.
    enum StoredArg {
        I32(i32),
        I64(i64),
        F32(f32),
        F64(f64),
        /// Owned C string passed as `char *`.  The `CString` keeps the allocation
        /// alive; `ptr` is the raw `*const c_char` handed to libffi.
        CStr(#[allow(dead_code)] CString, *const c_char),
        /// Owned C string passed as a generic `void *` (e.g. `Str` coerced to
        /// `CType::Pointer`).  The `CString` keeps the allocation alive.
        StrAsPtr(#[allow(dead_code)] CString, *const std::ffi::c_void),
        Ptr(*const std::ffi::c_void),
    }

    let mut stored: Vec<StoredArg> = Vec::with_capacity(arg_values.len());
    for (val, ctype) in arg_values.iter().zip(all_ctypes.iter()) {
        let s = match (val, ctype) {
            (DataValue::Int(n), CType::Int) => StoredArg::I32(*n as i32),
            (DataValue::Int(n), CType::Long | CType::SizeT) => StoredArg::I64(*n),
            (DataValue::Float(f), CType::Float) => StoredArg::F32(*f as f32),
            (DataValue::Float(f), CType::Double) => StoredArg::F64(*f),
            // Owned string → char *: the payload is already a CString, use directly.
            (DataValue::Str(s), CType::CString) => {
                let ptr = s.as_ptr();
                StoredArg::CStr(s.clone(), ptr)
            }
            // Unowned pointer passed where char * expected — pass raw address as-is.
            (DataValue::Pointer { addr, .. }, CType::CString) => {
                StoredArg::Ptr(*addr as *const std::ffi::c_void)
            }
            // Owned string coerced to a generic pointer type — allocate and pass addr.
            (DataValue::Str(s), CType::Pointer(_)) => {
                let cs = s.clone();
                let ptr = cs.as_ptr() as *const std::ffi::c_void;
                StoredArg::StrAsPtr(cs, ptr)
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
    let ffi_arg_types: Vec<libffi::middle::Type> =
        all_ctypes.iter().map(CType::to_ffi_type).collect();

    let mut ffi_args: Vec<libffi::middle::Arg> = Vec::with_capacity(stored.len());
    for s in &stored {
        let a = match s {
            StoredArg::I32(v) => libffi::middle::arg(v),
            StoredArg::I64(v) => libffi::middle::arg(v),
            StoredArg::F32(v) => libffi::middle::arg(v),
            StoredArg::F64(v) => libffi::middle::arg(v),
            // Pass `&ptr` so that libffi reads the char* value from the stack slot.
            StoredArg::CStr(_, ptr) => libffi::middle::arg(ptr),
            StoredArg::StrAsPtr(_, ptr) => libffi::middle::arg(ptr),
            StoredArg::Ptr(ptr) => libffi::middle::arg(ptr),
        };
        ffi_args.push(a);
    }

    let cif = if sig.variadic {
        Cif::new_variadic(ffi_arg_types, params.len(), sig.ret.to_ffi_type())
    } else {
        Cif::new(ffi_arg_types, sig.ret.to_ffi_type())
    };
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
            Some(DataValue::Pointer {
                addr: ptr as usize,
                pointee_ty: SidType::Str,
            })
        }
        CType::Pointer(pointee_ty) => {
            let ptr: *mut std::ffi::c_void = unsafe { cif.call(code_ptr, &ffi_args) };
            Some(DataValue::Pointer {
                addr: ptr as usize,
                pointee_ty: pointee_ty.clone(),
            })
        }
    };

    Ok(result)
}
