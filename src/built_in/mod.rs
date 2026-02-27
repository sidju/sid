use std::collections::HashMap;

use crate::InterpretBuiltIn;
use crate::CompileBuiltIn;

pub fn get_interpret_builtins() -> HashMap<&'static str, &'static dyn InterpretBuiltIn> {
  HashMap::new()
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