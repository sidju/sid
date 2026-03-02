use std::collections::HashMap;
use std::sync::Arc;

use crate::InterpretBuiltIn;
use crate::CompileBuiltIn;
use crate::DataValue;
use crate::c_ffi::{parse_c_header, load_c_functions};

// ── c_import ──────────────────────────────────────────────────────────────────

/// Built-in that reads a C header file and dynamically loads the corresponding
/// shared library, returning a `DataValue::Struct` whose fields are the
/// loaded C functions.
///
/// Argument: `DataValue::Struct([("header", Str(…)), ("lib", Str(…))])`.
/// The `header` field is the path to the `.h` file; `lib` is the path to
/// (or bare `soname` of) the shared library.
///
/// Return: `DataValue::Struct` mapping each bridgeable function name to a
/// `DataValue::CFunction`.
#[derive(Debug)]
struct CImport;

impl InterpretBuiltIn for CImport {
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
        "c_import expects a Struct {{header: str, lib: str}}, got {:?}", other
      ),
    };

    let get_str = |name: &str| -> anyhow::Result<String> {
      fields.iter()
        .find(|(k, _)| k == name)
        .and_then(|(_, v)| if let DataValue::Str(s) = v { Some(s.clone()) } else { None })
        .ok_or_else(|| anyhow::anyhow!("c_import: missing '{}' field (expected Str)", name))
    };

    let header_path = get_str("header")?;
    let lib_path = get_str("lib")?;

    let sigs = parse_c_header(&header_path)?;
    let funcs = load_c_functions(&lib_path, &sigs)?;

    let struct_fields: Vec<(String, DataValue)> = funcs
      .into_iter()
      .map(|f| {
        let name = f.name.clone();
        (name, DataValue::CFunction(Arc::new(f)))
      })
      .collect();

    Ok(Some(DataValue::Struct(struct_fields)))
  }
}

// ── Registry ──────────────────────────────────────────────────────────────────

pub fn get_interpret_builtins() -> HashMap<&'static str, &'static dyn InterpretBuiltIn> {
  static C_IMPORT: CImport = CImport;
  let mut m: HashMap<&'static str, &'static dyn InterpretBuiltIn> = HashMap::new();
  m.insert("c_import", &C_IMPORT);
  m
}

/// The subset of interpret builtins available during the comptime pass.
/// This is a deliberate design boundary: not every interpret builtin is
/// necessarily available at comptime.
pub fn get_comptime_builtins() -> HashMap<&'static str, &'static dyn InterpretBuiltIn> {
  HashMap::new()
}

/// Placeholder: compile builtins are registered here for use by the LLVM
/// backend. Each entry will also need a corresponding InterpretBuiltIn
/// implementation so the function can be exercised via the interpreter.
pub fn get_compile_builtins() -> HashMap<&'static str, &'static dyn CompileBuiltIn> {
  HashMap::new()
}