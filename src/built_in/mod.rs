use std::collections::HashMap;

use crate::InterpretBuiltIn;
use crate::CompileBuiltIn;
use crate::DataValue;
use crate::c_ffi::{CFuncSig, parse_c_header, link_sigs_to_lib};

// ── c_load_header ─────────────────────────────────────────────────────────────

/// Built-in that parses a C header file (via the system preprocessor) and
/// returns the extracted function signatures as a `DataValue::Struct` where
/// each field name is a function name and each value is a
/// `DataValue::CFuncSig` (unlinked — no library associated yet).
///
/// Argument: `DataValue::Str(path)` — path to the `.h` file.
/// Return:   `DataValue::Struct` of `(fn_name, CFuncSig)` pairs.
///
/// This builtin is available at **both** comptime and runtime.  Calling it
/// with `@!` at comptime bakes the type stubs into the compiled output so
/// that `c_link_lib` only needs to perform the dynamic-link step at runtime.
#[derive(Debug)]
struct CLoadHeader;

impl InterpretBuiltIn for CLoadHeader {
  fn arg_count(&self) -> u8 { 1 }
  fn return_count(&self) -> u8 { 1 }

  fn execute(
    &self,
    arg: Option<DataValue>,
    _global_scope: &HashMap<String, DataValue>,
  ) -> anyhow::Result<Option<DataValue>> {
    let path = match arg {
      Some(DataValue::Str(p)) => p,
      other => anyhow::bail!("c_load_header expects Str (header path), got {:?}", other),
    };
    let sigs = parse_c_header(&path)?;
    let fields: Vec<(String, DataValue)> = sigs
      .into_iter()
      .map(|s| {
        let name = s.name.clone();
        (name, DataValue::CFuncSig(s))
      })
      .collect();
    Ok(Some(DataValue::Struct(fields)))
  }
}

// ── c_link_lib ────────────────────────────────────────────────────────────────

/// Built-in that dynamically loads a shared library and associates it with
/// every `DataValue::CFuncSig` found in the supplied struct.
///
/// Argument: `DataValue::Struct([("header", Struct<CFuncSig>), ("lib", Str)])`.
/// The `header` field is the `DataValue::Struct` returned by `c_load_header`.
/// The `lib` field is the path to (or soname of) the shared library.
///
/// Return: `DataValue::Struct` with the same shape as `header` but each
/// `CFuncSig` whose symbol was found in the library now carries a library
/// handle.  Signatures whose symbol was **not** found are left unlinked so
/// that subsequent `c_link_lib` calls for other libraries can fulfil them.
///
/// When an invoked `CFuncSig` is called, the symbol is looked up in the
/// stored library by name at that point — no pre-resolved function pointer
/// is kept.
///
/// This builtin is **runtime-only** — dynamic library loading cannot happen
/// at comptime.
#[derive(Debug)]
struct CLinkLib;

impl InterpretBuiltIn for CLinkLib {
  fn arg_count(&self) -> u8 { 1 }
  fn return_count(&self) -> u8 { 1 }

  fn execute(
    &self,
    arg: Option<DataValue>,
    _global_scope: &HashMap<String, DataValue>,
  ) -> anyhow::Result<Option<DataValue>> {
    let fields = match arg {
      Some(DataValue::Struct(f)) => f,
      other => anyhow::bail!(
        "c_link_lib expects Struct {{header: Struct<CFuncSig>, lib: Str}}, got {:?}", other
      ),
    };

    // Extract `header` (a Struct of CFuncSig values).
    let header_fields = fields.iter()
      .find(|(k, _)| k == "header")
      .and_then(|(_, v)| if let DataValue::Struct(f) = v { Some(f.clone()) } else { None })
      .ok_or_else(|| anyhow::anyhow!(
        "c_link_lib: missing 'header' field (expected Struct from c_load_header)"
      ))?;

    // Extract `lib` path.
    let lib_path = fields.iter()
      .find(|(k, _)| k == "lib")
      .and_then(|(_, v)| if let DataValue::Str(s) = v { Some(s.clone()) } else { None })
      .ok_or_else(|| anyhow::anyhow!("c_link_lib: missing 'lib' field (expected Str)"))?;

    // Collect unlinked CFuncSig values from the header struct.
    let sigs: Vec<_> = header_fields.iter()
      .filter_map(|(_, v)| if let DataValue::CFuncSig(s) = v { Some(s.clone()) } else { None })
      .collect();

    // Link: check which symbols exist in the library and set `lib` on them.
    let linked = link_sigs_to_lib(&lib_path, &sigs)?;

    // Build a name → linked-sig map for safe lookup (non-CFuncSig entries in
    // the struct pass through unchanged).
    let linked_by_name: std::collections::HashMap<String, CFuncSig> = linked
      .into_iter()
      .map(|s| (s.name.clone(), s))
      .collect();

    let out_fields: Vec<(String, DataValue)> = header_fields.iter()
      .map(|(name, val)| {
        if matches!(val, DataValue::CFuncSig(_)) {
          if let Some(sig) = linked_by_name.get(name.as_str()) {
            return (name.clone(), DataValue::CFuncSig(sig.clone()));
          }
        }
        (name.clone(), val.clone())
      })
      .collect();

    Ok(Some(DataValue::Struct(out_fields)))
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