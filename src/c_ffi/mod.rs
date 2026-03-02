//! C foreign-function-interface helpers.
//!
//! This module provides:
//! - [`CType`]      — a small enum covering the C primitive types we can bridge
//! - [`CFuncSig`]   — a parsed C function signature (name + param/return types)
//! - [`CFunc`]      — a loaded, callable C function (library handle + pointer + sig)
//! - [`parse_c_header`] — a minimal line-oriented C header parser
//! - [`load_c_functions`] — load a shared library and resolve the supplied symbols
//! - [`call_c_function`]  — call a [`CFunc`] from a [`DataValue`] argument

use std::ffi::{CString, c_char};
use std::sync::Arc;

use anyhow::{bail, Result};
use libloading::Library;

use crate::DataValue;

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
    Pointer, // Any other pointer → DataValue::Int (raw address)
}

impl CType {
    /// Map a C type string (from the header) to a [`CType`].
    ///
    /// Qualifiers like `const`, `unsigned`, `restrict` are stripped before
    /// matching.
    pub fn from_c_str(raw: &str) -> Option<Self> {
        // Build a version with qualifiers and pointer characters removed for the
        // base-type lookup; keep the original to detect pointer types.
        let has_ptr = raw.contains('*');
        let s = raw
            .split_whitespace()
            .filter(|w| !matches!(
                *w,
                "const" | "volatile" | "restrict" | "unsigned" | "signed"
                | "extern" | "static" | "inline" | "__inline__" | "__restrict__"
            ))
            .collect::<Vec<_>>()
            .join(" ");
        let s = s.trim().trim_end_matches('*').trim();

        // Special-case char: bare `char` is Int; `char *` is CString.
        if s == "char" {
            return if has_ptr { Some(CType::CString) } else { Some(CType::Int) };
        }

        if has_ptr {
            return Some(CType::Pointer);
        }

        match s {
            "void" => Some(CType::Void),
            "int" | "short" | "short int" | "long int"
            | "int32_t" | "int16_t" | "int8_t"
            | "uint32_t" | "uint16_t" | "uint8_t"
            | "int64_t" | "uint64_t" => Some(CType::Int),
            "long" | "long long" | "long long int"
            | "ssize_t" | "ptrdiff_t" => Some(CType::Long),
            "size_t" => Some(CType::SizeT),
            "float" => Some(CType::Float),
            "double" | "long double" => Some(CType::Double),
            _ => None, // unknown — caller skips this function
        }
    }

    /// Map this [`CType`] to the corresponding libffi [`libffi::middle::Type`].
    pub fn to_ffi_type(&self) -> libffi::middle::Type {
        use libffi::middle::Type;
        match self {
            CType::Void => Type::void(),
            CType::Int => Type::i32(),
            CType::Long | CType::SizeT => Type::i64(),
            CType::Float => Type::f32(),
            CType::Double => Type::f64(),
            CType::CString | CType::Pointer => Type::pointer(),
        }
    }
}

// ── Parsed function signature ─────────────────────────────────────────────────

/// A parsed C function signature: return type, name, and ordered parameter types.
#[derive(Debug, Clone, PartialEq)]
pub struct CFuncSig {
    pub name: String,
    pub ret: CType,
    /// Parameter types in declaration order.  Unnamed parameters are fine.
    pub params: Vec<CType>,
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

// ── C header parsing ──────────────────────────────────────────────────────────

/// Parse a C header file and return all bridgeable function signatures.
///
/// The parser is intentionally minimal:
/// - preprocessor directives are discarded
/// - block and line comments are stripped
/// - only top-level `return_type name ( params ) ;` forms are recognised
/// - unknown / complex types (function pointers, arrays, variadic) are silently
///   skipped
pub fn parse_c_header(path: &str) -> Result<Vec<CFuncSig>> {
    let content = std::fs::read_to_string(path)?;
    let clean = strip_comments_and_directives(&content);
    let decls = split_declarations(&clean);
    let mut sigs = Vec::new();
    for decl in decls {
        if let Some(sig) = try_parse_func_decl(decl.trim()) {
            sigs.push(sig);
        }
    }
    Ok(sigs)
}

/// Remove `//` line comments, `/* */` block comments, and `#` preprocessor
/// directives from a C source string.
fn strip_comments_and_directives(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // Preprocessor directive: skip to end-of-line
            '#' => {
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if c == '\n' {
                        break;
                    }
                }
                out.push('\n');
            }
            '/' => match chars.peek() {
                Some(&'/') => {
                    // Line comment
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c == '\n' {
                            break;
                        }
                    }
                    out.push('\n');
                }
                Some(&'*') => {
                    // Block comment
                    chars.next(); // consume '*'
                    loop {
                        match chars.next() {
                            None => break,
                            Some('*') if chars.peek() == Some(&'/') => {
                                chars.next();
                                break;
                            }
                            Some('\n') => out.push('\n'),
                            _ => {}
                        }
                    }
                }
                _ => out.push('/'),
            },
            c => out.push(c),
        }
    }
    out
}

/// Split the pre-cleaned source into individual declarations using `;`.
fn split_declarations(src: &str) -> Vec<&str> {
    src.split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// Try to interpret `decl` as a C function declaration.
///
/// Returns `None` for anything that is not a recognisable function signature.
fn try_parse_func_decl(decl: &str) -> Option<CFuncSig> {
    // Must contain '(' and ')' to be a function declaration.
    if !decl.contains('(') || !decl.contains(')') {
        return None;
    }
    // Skip clearly non-function forms.
    for skip in &["typedef", "struct", "union", "enum", "=", "{", "(*"] {
        if decl.contains(skip) {
            return None;
        }
    }
    if decl.contains("__attribute__") || decl.contains("__declspec") {
        return None;
    }

    // Find the first `(` — everything before is `return_type function_name`.
    let open = decl.find('(')?;
    let close = decl.rfind(')')?;
    if close <= open {
        return None;
    }

    let before_paren = decl[..open].trim();
    let params_str = &decl[open + 1..close];

    // Function name: last identifier in before_paren.
    let name = before_paren
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|s| !s.is_empty())
        .last()?
        .to_owned();

    // Return type: everything before the name in before_paren.
    let name_start = before_paren.rfind(&name[..])?;
    let ret_str = before_paren[..name_start].trim();
    // Return type must be non-empty for a valid declaration.
    if ret_str.is_empty() {
        return None;
    }
    let ret = CType::from_c_str(ret_str)?;

    // Parse the parameter list.
    let params = parse_param_list(params_str)?;

    Some(CFuncSig { name, ret, params })
}

/// Parse a comma-separated C parameter list into a list of [`CType`]s.
///
/// Returns `None` if any parameter type is unknown or the list is malformed.
fn parse_param_list(params: &str) -> Option<Vec<CType>> {
    let params = params.trim();
    if params.is_empty() || params == "void" {
        return Some(vec![]);
    }
    // Skip variadic functions.
    if params.contains("...") {
        return None;
    }

    let mut result = Vec::new();
    for param in params.split(',') {
        let param = param.trim();
        if param.is_empty() {
            continue;
        }
        let type_str = strip_param_name(param);
        let ctype = CType::from_c_str(type_str)?;
        if ctype != CType::Void {
            result.push(ctype);
        }
    }
    Some(result)
}

/// Remove the parameter name from a C parameter declaration, leaving the type.
fn strip_param_name(param: &str) -> &str {
    let type_keywords = [
        "void", "char", "int", "short", "long", "float", "double",
        "unsigned", "signed", "const", "volatile", "restrict",
        "size_t", "ssize_t", "ptrdiff_t",
        "int8_t", "int16_t", "int32_t", "int64_t",
        "uint8_t", "uint16_t", "uint32_t", "uint64_t",
    ];
    let tokens: Vec<&str> = param.split_whitespace().collect();
    if tokens.len() <= 1 {
        return param;
    }
    let last = tokens[tokens.len() - 1];
    // Strip any leading `*` from `last` to isolate the identifier part.
    // This handles both `name` and `*name` (pointer-to-named-param) forms.
    let ident = last.trim_start_matches('*');
    let ident_clean = ident.trim_end_matches(|c: char| !c.is_alphanumeric() && c != '_');
    if !ident_clean.is_empty() && !type_keywords.contains(&ident_clean) {
        // The identifier portion is a parameter name — strip it.
        param[..param.len() - ident_clean.len()].trim()
    } else {
        param
    }
}

// ── Dynamic library loading ───────────────────────────────────────────────────

/// Load a shared library from `lib_path` and resolve every function listed in
/// `sigs`.  Symbols not found in the library are silently skipped.
///
/// # Safety
/// Loading and calling native libraries is inherently unsafe.
pub fn load_c_functions(lib_path: &str, sigs: &[CFuncSig]) -> Result<Vec<CFunc>> {
    // SAFETY: we're opening an existing shared library by path.
    let lib = unsafe { Library::new(lib_path) }
        .map_err(|e| anyhow::anyhow!("failed to load '{}': {}", lib_path, e))?;
    let lib = Arc::new(lib);

    let mut funcs = Vec::new();
    for sig in sigs {
        let sym_name = CString::new(sig.name.as_str()).unwrap();
        let fn_ptr: *const () = unsafe {
            // SAFETY: we're reading the function pointer value from the library;
            // we don't dereference it here.
            match lib.get::<unsafe extern "C" fn()>(sym_name.as_bytes_with_nul()) {
                Ok(sym) => *sym as *const (),
                Err(_) => continue, // symbol absent — skip
            }
        };
        funcs.push(CFunc {
            _lib: Arc::clone(&lib),
            name: sig.name.clone(),
            fn_ptr: FnPtr(fn_ptr),
            sig: sig.clone(),
        });
    }
    Ok(funcs)
}

// ── Calling a loaded C function ───────────────────────────────────────────────

/// Call `func` with the given `arg`.
///
/// - 0-param functions: pass `None`.
/// - 1-param functions: pass the single [`DataValue`].
/// - N-param functions: pass `DataValue::List` with items in declaration order.
///
/// # Safety
/// Calls arbitrary C code.  The caller must supply arguments matching the
/// declared C types.
pub fn call_c_function(func: &CFunc, arg: Option<DataValue>) -> Result<Option<DataValue>> {
    use libffi::middle::{Cif, CodePtr};

    let params = &func.sig.params;

    // Collect DataValue arguments.
    let arg_values: Vec<DataValue> = match (params.len(), arg) {
        (0, _) => vec![],
        (1, Some(v)) => vec![v],
        (n, Some(DataValue::List(items))) if items.len() == n => items,
        (n, _) => bail!(
            "CFunction '{}': expected {} argument(s)",
            func.name, n
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
                    "CFunction '{}': string argument contains interior NUL byte",
                    func.name
                ))?;
                let ptr = cs.as_ptr();
                StoredArg::CStr(cs, ptr)
            }
            (DataValue::Int(n), CType::Pointer) => {
                StoredArg::Ptr(*n as usize as *const std::ffi::c_void)
            }
            _ => bail!(
                "CFunction '{}': argument type mismatch \
                 (value {:?} vs expected C type {:?})",
                func.name, val, ctype
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

    let cif = Cif::new(ffi_arg_types, func.sig.ret.to_ffi_type());
    let code_ptr = CodePtr(func.fn_ptr.0 as *mut _);

    // SAFETY: We built the CIF to match the declared C signature.
    let result = match &func.sig.ret {
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
        CType::Pointer => {
            let ptr: *mut std::ffi::c_void = unsafe { cif.call(code_ptr, &ffi_args) };
            Some(DataValue::Int(ptr as i64))
        }
    };

    Ok(result)
}
