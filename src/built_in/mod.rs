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
/// Argument: `DataValue::Struct([("header", Str(path)), ("lib", Str(soname))])`.
/// Return:   `DataValue::Struct` of `(fn_name, CFuncSig)` pairs.
///
/// Both the header path and the library name must be provided together so that
/// `lib_name` is always populated — no separate link step is required before
/// invoking the returned functions.
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
    let fields = match arg {
      Some(DataValue::Struct(f)) => f,
      other => anyhow::bail!(
        "c_load_header expects Struct {{header: Str, lib: Str}}, got {:?}", other
      ),
    };

    let header_path = fields.iter()
      .find(|(k, _)| k == "header")
      .and_then(|(_, v)| if let DataValue::Str(s) = v { Some(s.clone()) } else { None })
      .ok_or_else(|| anyhow::anyhow!("c_load_header: missing 'header' field (expected Str)"))?;

    let lib_name = fields.iter()
      .find(|(k, _)| k == "lib")
      .and_then(|(_, v)| if let DataValue::Str(s) = v { Some(s.clone()) } else { None })
      .ok_or_else(|| anyhow::anyhow!("c_load_header: missing 'lib' field (expected Str)"))?;

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

// ── c_link_lib ────────────────────────────────────────────────────────────────

/// Built-in that pre-loads a shared library into [`GlobalState::libraries`].
///
/// This is optional: `call_cfuncsig` loads the library lazily on first call.
/// Use `c_link_lib` to get early error detection (the call fails immediately
/// if the library cannot be found) or to ensure a library is resident before
/// time-sensitive code runs.
///
/// Argument: `DataValue::Str(lib_path)` — path to (or soname of) the library.
/// Return:   nothing.
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
    let lib_path = match arg {
      Some(DataValue::Str(p)) => p,
      other => anyhow::bail!("c_link_lib expects Str (library path), got {:?}", other),
    };

    if !global_state.libraries.contains_key(lib_path.as_str()) {
      let lib = open_library(&lib_path)?;
      global_state.libraries.insert(lib_path, lib);
    }
    Ok(None)
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