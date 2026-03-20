use std::collections::HashMap;
use std::ffi::CString;

use crate::InterpretBuiltIn;
use crate::CompileBuiltIn;
use crate::DataValue;
use crate::GlobalState;
use crate::SidType;
use crate::c_ffi::{parse_c_header, open_library};

/// Convert a `CString` to a `String`, falling back to lossy UTF-8 conversion.
fn cstring_to_string(cs: CString) -> String {
  cs.into_string().unwrap_or_else(|e| e.into_cstring().to_string_lossy().into_owned())
}

// ── default scope ─────────────────────────────────────────────────────────────

/// Returns the default global scope, pre-populated with C-aligned type values.
///
/// Each entry maps a C-style type name to a `DataValue::Type(SidType)`, making
/// bare labels like `int`, `char`, etc. resolve to first-class type values.
pub fn default_scope() -> HashMap<String, DataValue> {
  let mut m = HashMap::new();
  for (name, ty) in [
    ("int",   SidType::Int),
    ("char",  SidType::Char),
    ("float", SidType::Float),
    ("str",   SidType::Str),
  ] {
    m.insert(name.to_owned(), DataValue::Type(ty));
  }
  m
}

// ── c_load_header ─────────────────────────────────────────────────────────────

/// Built-in that parses a C header file (via the system preprocessor) and
/// returns the extracted function signatures as a `DataValue::Struct` where
/// each field name is a function name and each value is a `DataValue::CFuncSig`
/// with `lib_name` already set.
///
/// Argument: either
///   - `DataValue::Str(header_path)` — lib_name derived from the header filename stem, or
///   - `DataValue::List([Str(header_path), Str(lib_name)])` — explicit lib_name override.
///
/// Return: `DataValue::Struct` of `(fn_name, CFuncSig)` pairs.
///
/// This builtin is available at **both** comptime and runtime.  Calling it
/// with `@!` at comptime bakes the type stubs into the compiled output so
/// that the library is loaded lazily on first call at runtime.
#[derive(Debug)]
struct CLoadHeader;

impl InterpretBuiltIn for CLoadHeader {
  fn arg_count(&self) -> u8 { 1 }
  fn return_count(&self) -> u8 { 1 }

  fn execute(
    &self,
    arg: Option<DataValue>,
    _global_state: &mut GlobalState<'_>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let (header_path, lib_name) = parse_load_header_arg(arg)?;
    let sigs = parse_c_header(&header_path, &lib_name)?;
    let out_fields: Vec<(String, DataValue)> = sigs
      .into_iter()
      .map(|s| {
        let name = s.name.clone();
        (name, DataValue::CFuncSig(s))
      })
      .collect();
    Ok(vec![DataValue::Struct(out_fields)])
  }
}

/// Parse the argument to `c_load_header` and return `(header_path, lib_name)`.
///
/// Accepts:
/// - `Str(path)` — lib_name derived from the filename stem.
/// - `List([Str(path), Str(lib_name)])` — explicit lib_name override.
fn parse_load_header_arg(arg: Option<DataValue>) -> anyhow::Result<(String, String)> {
  match arg {
    Some(DataValue::Str(path)) => {
      let lib_name = stem_of(&path.to_string_lossy())?;
      Ok((cstring_to_string(path), lib_name))
    }
    Some(DataValue::List(mut items)) if items.len() == 2 => {
      let path = match items.remove(0) {
        DataValue::Str(s) => cstring_to_string(s),
        other => anyhow::bail!("c_load_header: first list element must be Str (path), got {:?}", other),
      };
      let lib_name = match items.remove(0) {
        DataValue::Str(s) => cstring_to_string(s),
        other => anyhow::bail!("c_load_header: second list element must be Str (lib_name), got {:?}", other),
      };
      Ok((path, lib_name))
    }
    other => anyhow::bail!(
      "c_load_header expects Str(path) or [Str(path), Str(lib_name)], got {:?}", other
    ),
  }
}

/// Extract the filename stem (no suffix) from a path string.
fn stem_of(path: &str) -> anyhow::Result<String> {
  std::path::Path::new(path)
    .file_stem()
    .and_then(|s| s.to_str())
    .map(|s| s.to_owned())
    .ok_or_else(|| anyhow::anyhow!("c_load_header: could not derive lib_name from path '{}'", path))
}

// ── c_link_lib ────────────────────────────────────────────────────────────────

/// Built-in that pre-loads a shared library into [`GlobalState::libraries`].
///
/// Use `c_link_lib` to get early error detection (the call fails immediately
/// if the library cannot be found) and to ensure a library is registered under
/// a stable name before any `CFuncSig` that references it is invoked.
///
/// Argument: either
///   - `DataValue::Str(lib_path)` — load `lib_path`, register under that same path, or
///   - `DataValue::List([Str(lib_path), Str(lib_name)])` — load `lib_path`, register under `lib_name`.
///
/// Return: nothing.
///
/// This builtin is **runtime-only**.
#[derive(Debug)]
struct CLinkLib;

impl InterpretBuiltIn for CLinkLib {
  fn arg_count(&self) -> u8 { 1 }
  fn return_count(&self) -> u8 { 0 }

  fn execute(
    &self,
    arg: Option<DataValue>,
    global_state: &mut GlobalState<'_>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let (lib_path, lib_name) = parse_link_lib_arg(arg)?;
    if !global_state.libraries.contains_key(lib_name.as_str()) {
      let lib = open_library(&lib_path)?;
      global_state.libraries.insert(lib_name, lib);
    }
    Ok(vec![])
  }
}

/// Parse the argument to `c_link_lib` and return `(lib_path, lib_name)`.
///
/// Accepts:
/// - `Str(path)` — lib_path and lib_name are both the path.
/// - `List([Str(path), Str(name)])` — load path, register under name.
fn parse_link_lib_arg(arg: Option<DataValue>) -> anyhow::Result<(String, String)> {
  match arg {
    Some(DataValue::Str(path)) => Ok((cstring_to_string(path.clone()), cstring_to_string(path))),
    Some(DataValue::List(mut items)) if items.len() == 2 => {
      let path = match items.remove(0) {
        DataValue::Str(s) => cstring_to_string(s),
        other => anyhow::bail!("c_link_lib: first list element must be Str (lib_path), got {:?}", other),
      };
      let name = match items.remove(0) {
        DataValue::Str(s) => cstring_to_string(s),
        other => anyhow::bail!("c_link_lib: second list element must be Str (lib_name), got {:?}", other),
      };
      Ok((path, name))
    }
    other => anyhow::bail!("c_link_lib expects Str(lib_path) or [Str(lib_path), Str(lib_name)], got {:?}", other),
  }
}

// ── load_scope ────────────────────────────────────────────────────────────────

/// Built-in that unpacks a `DataValue::Struct` into the global scope.
///
/// Each field `(name, value)` in the struct is inserted into
/// [`GlobalState::scope`] under `name`, making it directly addressable as a
/// label in subsequent code.
///
/// This is the standard way to bring the function signatures returned by
/// `c_load_header` into scope so they can be invoked by name:
///
/// ```text
/// "/usr/include/stdio.h" c_load_header @!
/// load_scope !
/// "hello\n" fputs !
/// ```
///
/// Argument: `DataValue::Struct(fields)`.
/// Return:   nothing.
///
/// This builtin is available at **both** comptime and runtime.
#[derive(Debug)]
struct LoadScope;

impl InterpretBuiltIn for LoadScope {
  fn arg_count(&self) -> u8 { 1 }
  fn return_count(&self) -> u8 { 0 }

  fn execute(
    &self,
    arg: Option<DataValue>,
    global_state: &mut GlobalState<'_>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let fields = match arg {
      Some(DataValue::Struct(f)) => f,
      other => anyhow::bail!("load_scope expects Struct, got {:?}", other),
    };
    for (name, value) in fields {
      global_state.scope.insert(name, value);
    }
    Ok(vec![])
  }
}

// ── clone ─────────────────────────────────────────────────────────────────────

/// Built-in that duplicates the top-of-stack value.
///
/// Pops one value and pushes it back twice, leaving two copies on the stack.
/// Useful when a value must be passed to a function that consumes it but
/// also needed again afterwards — for example, a `FILE*` pointer that should
/// persist across multiple C calls.
///
/// Argument: any `DataValue`.
/// Return:   the value twice.
///
/// This builtin is **runtime-only**.
#[derive(Debug)]
struct Clone;

impl InterpretBuiltIn for Clone {
  fn arg_count(&self) -> u8 { 1 }
  fn return_count(&self) -> u8 { 2 }

  fn execute(
    &self,
    arg: Option<DataValue>,
    _global_state: &mut GlobalState<'_>,
  ) -> anyhow::Result<Vec<DataValue>> {
    match arg {
      Some(v) => Ok(vec![v.clone(), v]),
      None => anyhow::bail!("clone: expected a value but got nothing"),
    }
  }
}

// ── drop ──────────────────────────────────────────────────────────────────────

/// Built-in that discards the top-of-stack value.
///
/// Useful for consuming return values from C functions that are not needed
/// (e.g. the `int` returned by `puts` or `fclose`).
///
/// Argument: any `DataValue`.
/// Return:   nothing.
///
/// This builtin is **runtime-only**.
#[derive(Debug)]
struct Drop;

impl InterpretBuiltIn for Drop {
  fn arg_count(&self) -> u8 { 1 }
  fn return_count(&self) -> u8 { 0 }

  fn execute(
    &self,
    _arg: Option<DataValue>,
    _global_state: &mut GlobalState<'_>,
  ) -> anyhow::Result<Vec<DataValue>> {
    Ok(vec![])
  }
}

// ── eq ────────────────────────────────────────────────────────────────────────

/// Built-in that tests two values for equality.
///
/// Argument: `DataValue::List([a, b])`.
/// Return:   `DataValue::Bool(a == b)`.
///
/// This builtin is **runtime-only**.
#[derive(Debug)]
struct Eq;

impl InterpretBuiltIn for Eq {
  fn arg_count(&self) -> u8 { 1 }
  fn return_count(&self) -> u8 { 1 }

  fn execute(
    &self,
    arg: Option<DataValue>,
    _global_state: &mut GlobalState<'_>,
  ) -> anyhow::Result<Vec<DataValue>> {
    match arg {
      Some(DataValue::List(mut items)) if items.len() == 2 => {
        let b = items.remove(1);
        let a = items.remove(0);
        Ok(vec![DataValue::Bool(a == b)])
      }
      other => anyhow::bail!("eq expects [a, b], got {:?}", other),
    }
  }
}

// ── assert ────────────────────────────────────────────────────────────────────

/// Built-in that asserts a condition is true, aborting with an error if not.
///
/// Argument: `DataValue::Bool(condition)`.
/// Return:   nothing on success; returns `Err` if the condition is false.
///
/// This builtin is **runtime-only**.
#[derive(Debug)]
struct Assert;

impl InterpretBuiltIn for Assert {
  fn arg_count(&self) -> u8 { 1 }
  fn return_count(&self) -> u8 { 0 }

  fn execute(
    &self,
    arg: Option<DataValue>,
    _global_state: &mut GlobalState<'_>,
  ) -> anyhow::Result<Vec<DataValue>> {
    match arg {
      Some(DataValue::Bool(true)) => Ok(vec![]),
      Some(DataValue::Bool(false)) => anyhow::bail!("assertion failed"),
      other => anyhow::bail!("assert expects Bool, got {:?}", other),
    }
  }
}

// ── ptr_cast ──────────────────────────────────────────────────────────────────

/// Built-in that re-types a pointer by replacing its pointee type.
///
/// Useful for casting a `void *` returned by `malloc` (or any untyped pointer)
/// to a concrete pointer type before passing it to a C function that expects a
/// typed pointer.
///
/// Argument: `DataValue::List([Pointer{addr, ..}, Type(SidType)])`.
/// Return:   `DataValue::Pointer { addr, pointee_ty }` with the new type.
///
/// Example (cast `void *` from malloc to a `str` pointer):
/// ```text
/// 4096 malloc!  [$ Str ptr]  ptr_cast!
/// ```
#[derive(Debug)]
struct PtrCast;

impl InterpretBuiltIn for PtrCast {
  fn arg_count(&self) -> u8 { 2 }
  fn return_count(&self) -> u8 { 1 }

  fn execute(
    &self,
    arg: Option<DataValue>,
    _global_state: &mut GlobalState<'_>,
  ) -> anyhow::Result<Vec<DataValue>> {
    match arg {
      Some(DataValue::List(mut items)) if items.len() == 2 => {
        let addr = match items.remove(0) {
          DataValue::Pointer { addr, .. } => addr,
          other => anyhow::bail!("ptr_cast: first element must be a Pointer, got {:?}", other),
        };
        let pointee_ty = match items.remove(0) {
          DataValue::Type(ty) => ty,
          other => anyhow::bail!("ptr_cast: second element must be a Type, got {:?}", other),
        };
        Ok(vec![DataValue::Pointer { addr, pointee_ty }])
      }
      other => anyhow::bail!("ptr_cast expects [Pointer, Type], got {:?}", other),
    }
  }
}

// ── ptr_read_cstr ─────────────────────────────────────────────────────────────

/// Built-in that reads a null-terminated C string from a raw pointer.
///
/// The pointer must point to a region of memory that contains a valid
/// null-terminated byte string.  The bytes are copied into a new
/// `DataValue::Str` (backed by `CString`) — the original buffer is not freed
/// or otherwise affected.
///
/// Argument: `DataValue::Pointer { addr, .. }`.
/// Return:   `DataValue::Str(CString)` with the contents up to the first NUL.
///
/// # Safety
/// The pointer must be non-null and point to a valid, null-terminated C string.
/// Behaviour is undefined if `addr` is dangling, misaligned, or not
/// null-terminated.
#[derive(Debug)]
struct PtrReadCstr;

impl InterpretBuiltIn for PtrReadCstr {
  fn arg_count(&self) -> u8 { 1 }
  fn return_count(&self) -> u8 { 1 }

  fn execute(
    &self,
    arg: Option<DataValue>,
    _global_state: &mut GlobalState<'_>,
  ) -> anyhow::Result<Vec<DataValue>> {
    match arg {
      Some(DataValue::Pointer { addr, .. }) => {
        let ptr = addr as *const std::ffi::c_char;
        if ptr.is_null() {
          anyhow::bail!("ptr_read_cstr: pointer is null");
        }
        // SAFETY: caller guarantees the pointer is valid and null-terminated.
        let cs = unsafe { std::ffi::CStr::from_ptr(ptr) }.to_owned();
        Ok(vec![DataValue::Str(cs)])
      }
      other => anyhow::bail!("ptr_read_cstr expects Pointer, got {:?}", other),
    }
  }
}

pub fn get_interpret_builtins() -> HashMap<&'static str, &'static dyn InterpretBuiltIn> {
  static C_LOAD_HEADER: CLoadHeader = CLoadHeader;
  static C_LINK_LIB: CLinkLib = CLinkLib;
  static LOAD_SCOPE: LoadScope = LoadScope;
  static CLONE: Clone = Clone;
  static DROP: Drop = Drop;
  static EQ: Eq = Eq;
  static ASSERT: Assert = Assert;
  static PTR_CAST: PtrCast = PtrCast;
  static PTR_READ_CSTR: PtrReadCstr = PtrReadCstr;
  let mut m: HashMap<&'static str, &'static dyn InterpretBuiltIn> = HashMap::new();
  m.insert("c_load_header", &C_LOAD_HEADER);
  m.insert("c_link_lib", &C_LINK_LIB);
  m.insert("load_scope", &LOAD_SCOPE);
  m.insert("clone", &CLONE);
  m.insert("drop", &DROP);
  m.insert("eq", &EQ);
  m.insert("assert", &ASSERT);
  m.insert("ptr_cast", &PTR_CAST);
  m.insert("ptr_read_cstr", &PTR_READ_CSTR);
  m
}

/// The subset of interpret builtins available during the comptime pass.
/// `c_load_header` and `load_scope` are available at comptime so headers can
/// be parsed once and the type stubs embedded in the compiled output, with
/// their names brought directly into scope.
pub fn get_comptime_builtins() -> HashMap<&'static str, &'static dyn InterpretBuiltIn> {
  static C_LOAD_HEADER: CLoadHeader = CLoadHeader;
  static LOAD_SCOPE: LoadScope = LoadScope;
  let mut m: HashMap<&'static str, &'static dyn InterpretBuiltIn> = HashMap::new();
  m.insert("c_load_header", &C_LOAD_HEADER);
  m.insert("load_scope", &LOAD_SCOPE);
  m
}

/// Placeholder: compile builtins are registered here for use by the LLVM
/// backend.
pub fn get_compile_builtins() -> HashMap<&'static str, &'static dyn CompileBuiltIn> {
  HashMap::new()
}