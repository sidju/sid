use std::collections::HashMap;

use crate::InterpretBuiltIn;
use crate::CompileBuiltIn;
use crate::DataValue;
use crate::GlobalState;
use crate::c_ffi::{parse_c_header, open_library};

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
    _global_state: &mut GlobalState,
  ) -> anyhow::Result<Option<DataValue>> {
    let (header_path, lib_name) = parse_load_header_arg(arg)?;
    let sigs = parse_c_header(&header_path, &lib_name)?;
    let out_fields: Vec<(String, DataValue)> = sigs
      .into_iter()
      .map(|s| {
        let name = s.name.clone();
        (name, DataValue::CFuncSig(s))
      })
      .collect();
    Ok(Some(DataValue::Struct(out_fields)))
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
      let lib_name = stem_of(&path)?;
      Ok((path, lib_name))
    }
    Some(DataValue::List(mut items)) if items.len() == 2 => {
      let path = match items.remove(0) {
        DataValue::Str(s) => s,
        other => anyhow::bail!("c_load_header: first list element must be Str (path), got {:?}", other),
      };
      let lib_name = match items.remove(0) {
        DataValue::Str(s) => s,
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
    global_state: &mut GlobalState,
  ) -> anyhow::Result<Option<DataValue>> {
    let (lib_path, lib_name) = parse_link_lib_arg(arg)?;
    if !global_state.libraries.contains_key(lib_name.as_str()) {
      let lib = open_library(&lib_path)?;
      global_state.libraries.insert(lib_name, lib);
    }
    Ok(None)
  }
}

/// Parse the argument to `c_link_lib` and return `(lib_path, lib_name)`.
///
/// Accepts:
/// - `Str(path)` — lib_path and lib_name are both the path.
/// - `List([Str(path), Str(name)])` — load path, register under name.
fn parse_link_lib_arg(arg: Option<DataValue>) -> anyhow::Result<(String, String)> {
  match arg {
    Some(DataValue::Str(path)) => Ok((path.clone(), path)),
    Some(DataValue::List(mut items)) if items.len() == 2 => {
      let path = match items.remove(0) {
        DataValue::Str(s) => s,
        other => anyhow::bail!("c_link_lib: first list element must be Str (lib_path), got {:?}", other),
      };
      let name = match items.remove(0) {
        DataValue::Str(s) => s,
        other => anyhow::bail!("c_link_lib: second list element must be Str (lib_name), got {:?}", other),
      };
      Ok((path, name))
    }
    other => anyhow::bail!("c_link_lib expects Str(lib_path) or [Str(lib_path), Str(lib_name)], got {:?}", other),
  }
}

// ── Registry ──────────────────────────────────────────────────────────────────

pub fn get_interpret_builtins() -> HashMap<&'static str, &'static dyn InterpretBuiltIn> {
  static C_LOAD_HEADER: CLoadHeader = CLoadHeader;
  static C_LINK_LIB: CLinkLib = CLinkLib;
  let mut m: HashMap<&'static str, &'static dyn InterpretBuiltIn> = HashMap::new();
  m.insert("c_load_header", &C_LOAD_HEADER);
  m.insert("c_link_lib", &C_LINK_LIB);
  m
}

/// The subset of interpret builtins available during the comptime pass.
/// `c_load_header` is available at comptime so headers can be parsed once
/// and the type stubs embedded in the compiled output.
pub fn get_comptime_builtins() -> HashMap<&'static str, &'static dyn InterpretBuiltIn> {
  static C_LOAD_HEADER: CLoadHeader = CLoadHeader;
  let mut m: HashMap<&'static str, &'static dyn InterpretBuiltIn> = HashMap::new();
  m.insert("c_load_header", &C_LOAD_HEADER);
  m
}

/// Placeholder: compile builtins are registered here for use by the LLVM
/// backend.
pub fn get_compile_builtins() -> HashMap<&'static str, &'static dyn CompileBuiltIn> {
  HashMap::new()
}