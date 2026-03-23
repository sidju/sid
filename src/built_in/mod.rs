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

/// Pop and unwrap the top concrete `DataValue` from the data stack.
fn pop_arg(
  data_stack: &mut Vec<crate::TemplateValue>,
  builtin_name: &str,
) -> anyhow::Result<DataValue> {
  use crate::{TemplateValue, ProgramValue};
  match data_stack.pop() {
    Some(TemplateValue::Literal(ProgramValue::Data(v))) => Ok(v),
    Some(other) => anyhow::bail!(
      "{}: argument is not a concrete value: {:?}", builtin_name, other
    ),
    None => anyhow::bail!(
      "{}: expected an argument but the stack was empty", builtin_name
    ),
  }
}

// ── default scope ─────────────────────────────────────────────────────────────

/// Returns the default global scope, pre-populated with C-aligned type values.
///
/// Each entry maps a C-style type name to a `DataValue::Type(SidType)`, making
/// bare labels like `int`, `char`, etc. resolve to first-class type values.
pub fn default_scope() -> HashMap<String, DataValue> {
  let mut m = HashMap::new();
  for (name, ty) in [
    ("bool",  SidType::Bool),
    ("int",   SidType::Int),
    ("char",  SidType::Char),
    ("float", SidType::Float),
    ("str",   SidType::Str),
    ("Any",   SidType::Any),
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
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let arg = pop_arg(data_stack, "c_load_header")?;
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
fn parse_load_header_arg(arg: DataValue) -> anyhow::Result<(String, String)> {
  match arg {
    DataValue::Str(path) => {
      let lib_name = stem_of(&path.to_string_lossy())?;
      Ok((cstring_to_string(path), lib_name))
    }
    DataValue::List(mut items) if items.len() == 2 => {
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
/// Argument: either
///   - `DataValue::Str(lib_path)` — load `lib_path`, register under that same path, or
///   - `DataValue::List([Str(lib_path), Str(lib_name)])` — load `lib_path`, register under `lib_name`.
///
/// Return: nothing.
#[derive(Debug)]
struct CLinkLib;

impl InterpretBuiltIn for CLinkLib {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let arg = pop_arg(data_stack, "c_link_lib")?;
    let (lib_path, lib_name) = parse_link_lib_arg(arg)?;
    if !global_state.libraries.contains_key(lib_name.as_str()) {
      let lib = open_library(&lib_path)?;
      global_state.libraries.insert(lib_name, lib);
    }
    Ok(vec![])
  }
}

/// Parse the argument to `c_link_lib` and return `(lib_path, lib_name)`.
fn parse_link_lib_arg(arg: DataValue) -> anyhow::Result<(String, String)> {
  match arg {
    DataValue::Str(path) => Ok((cstring_to_string(path.clone()), cstring_to_string(path))),
    DataValue::List(mut items) if items.len() == 2 => {
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
/// Argument: `DataValue::Struct(fields)`.
/// Return:   nothing.
///
/// This builtin is available at **both** comptime and runtime.
#[derive(Debug)]
struct LoadScope;

impl InterpretBuiltIn for LoadScope {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let fields = match pop_arg(data_stack, "load_scope")? {
      DataValue::Struct(f) => f,
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
/// Pops one value and pushes it back twice.
#[derive(Debug)]
struct Clone;

impl InterpretBuiltIn for Clone {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let v = pop_arg(data_stack, "clone")?;
    Ok(vec![v.clone(), v])
  }
}

// ── drop ──────────────────────────────────────────────────────────────────────

/// Built-in that discards the top-of-stack value.
#[derive(Debug)]
struct Drop;

impl InterpretBuiltIn for Drop {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    pop_arg(data_stack, "drop")?;
    Ok(vec![])
  }
}

// ── eq ────────────────────────────────────────────────────────────────────────

/// Built-in that tests two values for equality.
///
/// Pops two values and returns `DataValue::Bool(a == b)`.
/// The top of the stack is the right-hand side, the value below is the left.
#[derive(Debug)]
struct Eq;

impl InterpretBuiltIn for Eq {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let b = pop_arg(data_stack, "eq")?;
    let a = pop_arg(data_stack, "eq")?;
    Ok(vec![DataValue::Bool(a == b)])
  }
}

// ── assert ────────────────────────────────────────────────────────────────────

/// Built-in that asserts a condition is true, aborting with an error if not.
///
/// Argument: `DataValue::Bool(condition)`.
/// Return:   nothing on success; returns `Err` if the condition is false.
#[derive(Debug)]
struct Assert;

impl InterpretBuiltIn for Assert {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    match pop_arg(data_stack, "assert")? {
      DataValue::Bool(true)  => Ok(vec![]),
      DataValue::Bool(false) => anyhow::bail!("assertion failed"),
      other => anyhow::bail!("assert expects Bool, got {:?}", other),
    }
  }
}

// ── not ───────────────────────────────────────────────────────────────────────

/// Built-in that negates a boolean value.
///
/// Argument: `DataValue::Bool(b)`.
/// Return:   `DataValue::Bool(!b)`.
#[derive(Debug)]
struct Not;

impl InterpretBuiltIn for Not {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    match pop_arg(data_stack, "not")? {
      DataValue::Bool(b) => Ok(vec![DataValue::Bool(!b)]),
      other => anyhow::bail!("not expects Bool, got {:?}", other),
    }
  }
}

// ── null ──────────────────────────────────────────────────────────────────────

/// Built-in that pushes a null pointer (`Pointer { addr: 0, pointee_ty: Any }`).
///
/// Useful for comparing C function return values against NULL, e.g.:
/// ```text
/// fgets_result  null  eq!  not!  assert!
/// ```
#[derive(Debug)]
struct Null;

impl InterpretBuiltIn for Null {
  fn execute(
    &self,
    _data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    Ok(vec![DataValue::Pointer { addr: 0, pointee_ty: SidType::Any }])
  }
}

// ── ptr_cast ──────────────────────────────────────────────────────────────────

/// Built-in that re-types a pointer by replacing its pointee type.
///
/// Pops two values: the pointer (deeper) then the new type (top).
/// Returns a new `Pointer` with the given pointee type.
///
/// Usage: `malloc_result  str  ptr_cast!`
#[derive(Debug)]
struct PtrCast;

impl InterpretBuiltIn for PtrCast {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let new_type = pop_arg(data_stack, "ptr_cast")?;
    let pointer  = pop_arg(data_stack, "ptr_cast")?;
    let addr = match pointer {
      DataValue::Pointer { addr, .. } => addr,
      other => anyhow::bail!("ptr_cast: first argument must be a Pointer, got {:?}", other),
    };
    let pointee_ty = match new_type {
      DataValue::Type(ty) => ty,
      DataValue::Label(name) => match global_state.scope.get(&name) {
        Some(DataValue::Type(ty)) => ty.clone(),
        Some(other) => anyhow::bail!("ptr_cast: label '{}' resolves to {:?}, not a Type", name, other),
        None => anyhow::bail!("ptr_cast: undefined label '{}'", name),
      },
      other => anyhow::bail!("ptr_cast: type argument must be a Type or label, got {:?}", other),
    };
    Ok(vec![DataValue::Pointer { addr, pointee_ty }])
  }
}

// ── ptr_read_cstr ─────────────────────────────────────────────────────────────

/// Built-in that reads a null-terminated C string from a raw pointer.
///
/// Pops a `Pointer`, reads bytes up to the first NUL, and returns
/// `DataValue::Str(CString)`.
///
/// # Safety
/// The pointer must be non-null and point to a valid null-terminated C string.
#[derive(Debug)]
struct PtrReadCstr;

impl InterpretBuiltIn for PtrReadCstr {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    match pop_arg(data_stack, "ptr_read_cstr")? {
      DataValue::Pointer { addr, .. } => {
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

// ── debug_stack ───────────────────────────────────────────────────────────────

/// Built-in that prints the top N values of the data stack without consuming them.
///
/// Pops an `Int(n)`, peeks at the top `n` remaining entries, prints them to
/// stderr, then returns nothing (the peeked entries stay on the stack).
#[derive(Debug)]
struct DebugStack;

impl InterpretBuiltIn for DebugStack {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let n = match pop_arg(data_stack, "debug_stack")? {
      DataValue::Int(n) if n >= 0 => n as usize,
      DataValue::Int(n) => anyhow::bail!("debug_stack: count must be non-negative, got {}", n),
      other => anyhow::bail!("debug_stack expects Int, got {:?}", other),
    };
    let len = data_stack.len();
    let start = len.saturating_sub(n);
    eprintln!("=== debug_stack (top {} of {}) ===", n.min(len), len);
    for entry in data_stack[start..].iter().rev() {
      eprintln!("  {:?}", entry);
    }
    Ok(vec![])
  }
}

// ── while_do ──────────────────────────────────────────────────────────────────

/// Built-in that checks a condition first, then loops the body while it holds.
///
/// Usage: `state... (cond_substack) (body_substack) while_do !`
/// Reads naturally as "while `cond`, do `body`": condition is below body on
/// the stack, matching the left-to-right reading order of the call.
///
/// Schedules `cond → CondLoop`. Each time `CondLoop` fires and the condition
/// is true, it re-schedules `body → cond → CondLoop`; on false the loop exits.
/// The body may never run if the condition is false on the first check.
///
/// Stack contract:
///   - Condition: net +1 (leaves one `Bool` on top, all other items unchanged).
///   - Body: net 0 (leaves the stack exactly as it found it).
#[derive(Debug)]
struct WhileDo;

impl InterpretBuiltIn for WhileDo {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let body_val = pop_arg(data_stack, "while_do")?;
    let cond_val = pop_arg(data_stack, "while_do")?;
    let body = match body_val {
      DataValue::Substack { body: s, .. } => s,
      other => anyhow::bail!("while_do: body must be a Substack, got {:?}", other),
    };
    let cond = match cond_val {
      DataValue::Substack { body: s, .. } => s,
      other => anyhow::bail!("while_do: condition must be a Substack, got {:?}", other),
    };
    let expected_len = data_stack.len();
    // Schedule: cond runs first, then CondLoop checks the bool.
    program_stack.push(crate::ProgramValue::CondLoop { cond: cond.clone(), body, expected_len });
    let mut cond_rev: Vec<crate::ProgramValue> = cond.iter().rev().cloned().collect();
    program_stack.append(&mut cond_rev);
    Ok(vec![])
  }
}

// ── do_while ──────────────────────────────────────────────────────────────────

/// Built-in that runs the body once unconditionally, then loops while the
/// condition holds.
///
/// Usage: `state... (body_substack) (cond_substack) do_while !`
/// Reads naturally as "do `body`, while `cond`": body is below condition on
/// the stack, matching the left-to-right reading order of the call.
///
/// Schedules `body → cond → CondLoop` so the body always executes at least
/// once. Subsequent iterations behave identically to `while_do`.
///
/// Stack contract:
///   - Condition: net +1 (leaves one `Bool` on top, all other items unchanged).
///   - Body: net 0 (leaves the stack exactly as it found it).
#[derive(Debug)]
struct DoWhile;

impl InterpretBuiltIn for DoWhile {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let cond_val = pop_arg(data_stack, "do_while")?;
    let body_val = pop_arg(data_stack, "do_while")?;
    let cond = match cond_val {
      DataValue::Substack { body: s, .. } => s,
      other => anyhow::bail!("do_while: condition must be a Substack, got {:?}", other),
    };
    let body = match body_val {
      DataValue::Substack { body: s, .. } => s,
      other => anyhow::bail!("do_while: body must be a Substack, got {:?}", other),
    };
    let expected_len = data_stack.len();
    // Schedule: body runs first, then StackSizeAssert checks it, then cond, then CondLoop.
    program_stack.push(crate::ProgramValue::CondLoop { cond: cond.clone(), body: body.clone(), expected_len });
    let mut cond_rev: Vec<crate::ProgramValue> = cond.iter().rev().cloned().collect();
    let mut body_rev: Vec<crate::ProgramValue> = body.iter().rev().cloned().collect();
    program_stack.append(&mut cond_rev);
    program_stack.push(crate::ProgramValue::StackSizeAssert {
      expected_len,
      message: "loop body must leave the stack unchanged",
    });
    program_stack.append(&mut body_rev);
    Ok(vec![])
  }
}

// ── fn / typed_args / typed_rets / untyped_args / untyped_rets ───────────────

/// Extracts a `Vec<SidType>` from a `DataValue::List` where every element is
/// `DataValue::Type(...)`. Returns an error if any element is not a type.
fn list_to_type_vec(list: DataValue, ctx: &str) -> anyhow::Result<Vec<SidType>> {
  match list {
    DataValue::List(items) => items.into_iter().map(|v| match v {
      DataValue::Type(t) => Ok(t),
      other => anyhow::bail!("{}: expected a list of types, got {:?}", ctx, other),
    }).collect(),
    other => anyhow::bail!("{}: expected a List of types, got {:?}", ctx, other),
  }
}

/// Pushes `DataValue::Type(SidType::Fn { args: None, ret: None })` — an
/// unconstrained callable type. Use `typed_args`/`typed_rets` to narrow it.
#[derive(Debug)]
struct FnType;

impl InterpretBuiltIn for FnType {
  fn execute(
    &self,
    _data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    Ok(vec![DataValue::Type(SidType::Fn { args: None, ret: None })])
  }
}

/// Sets the `args` type annotation on a `Substack`/`Script` or `SidType::Fn`.
///
/// Usage: `callable [T1 T2 …] typed_args !`
/// Pops the list of types (top), then the callable; returns it with args set.
#[derive(Debug)]
struct TypedArgs;

impl InterpretBuiltIn for TypedArgs {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let types_val  = pop_arg(data_stack, "typed_args")?;
    let target_val = pop_arg(data_stack, "typed_args")?;
    let types = list_to_type_vec(types_val, "typed_args")?;
    match target_val {
      DataValue::Substack { body, ret, .. } =>
        Ok(vec![DataValue::Substack { body, args: Some(types), ret }]),
      DataValue::Script { body, ret, .. } =>
        Ok(vec![DataValue::Script { body, args: Some(types), ret }]),
      DataValue::Type(SidType::Fn { ret, .. }) =>
        Ok(vec![DataValue::Type(SidType::Fn { args: Some(types), ret })]),
      other => anyhow::bail!("typed_args: expected Substack, Script, or Fn type, got {:?}", other),
    }
  }
}

/// Sets the `ret` type annotation on a `Substack`/`Script` or `SidType::Fn`.
///
/// Usage: `callable [T1 T2 …] typed_rets !`
#[derive(Debug)]
struct TypedRets;

impl InterpretBuiltIn for TypedRets {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let types_val  = pop_arg(data_stack, "typed_rets")?;
    let target_val = pop_arg(data_stack, "typed_rets")?;
    let types = list_to_type_vec(types_val, "typed_rets")?;
    match target_val {
      DataValue::Substack { body, args, .. } =>
        Ok(vec![DataValue::Substack { body, args, ret: Some(types) }]),
      DataValue::Script { body, args, .. } =>
        Ok(vec![DataValue::Script { body, args, ret: Some(types) }]),
      DataValue::Type(SidType::Fn { args, .. }) =>
        Ok(vec![DataValue::Type(SidType::Fn { args, ret: Some(types) })]),
      other => anyhow::bail!("typed_rets: expected Substack, Script, or Fn type, got {:?}", other),
    }
  }
}

/// Clears the `args` type annotation (sets it to `None`) on a `Substack`/`Script`
/// or `SidType::Fn`.
///
/// Usage: `callable untyped_args !`
#[derive(Debug)]
struct UntypedArgs;

impl InterpretBuiltIn for UntypedArgs {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let target_val = pop_arg(data_stack, "untyped_args")?;
    match target_val {
      DataValue::Substack { body, ret, .. } =>
        Ok(vec![DataValue::Substack { body, args: None, ret }]),
      DataValue::Script { body, ret, .. } =>
        Ok(vec![DataValue::Script { body, args: None, ret }]),
      DataValue::Type(SidType::Fn { ret, .. }) =>
        Ok(vec![DataValue::Type(SidType::Fn { args: None, ret })]),
      other => anyhow::bail!("untyped_args: expected Substack, Script, or Fn type, got {:?}", other),
    }
  }
}

/// Clears the `ret` type annotation (sets it to `None`) on a `Substack`/`Script`
/// or `SidType::Fn`.
///
/// Usage: `callable untyped_rets !`
#[derive(Debug)]
struct UntypedRets;

impl InterpretBuiltIn for UntypedRets {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let target_val = pop_arg(data_stack, "untyped_rets")?;
    match target_val {
      DataValue::Substack { body, args, .. } =>
        Ok(vec![DataValue::Substack { body, args, ret: None }]),
      DataValue::Script { body, args, .. } =>
        Ok(vec![DataValue::Script { body, args, ret: None }]),
      DataValue::Type(SidType::Fn { args, .. }) =>
        Ok(vec![DataValue::Type(SidType::Fn { args, ret: None })]),
      other => anyhow::bail!("untyped_rets: expected Substack, Script, or Fn type, got {:?}", other),
    }
  }
}

// Module-level statics so both get_interpret_builtins and get_comptime_builtins
// can reference them without duplicating declarations.
static C_LOAD_HEADER: CLoadHeader = CLoadHeader;
static C_LINK_LIB:    CLinkLib    = CLinkLib;
static LOAD_SCOPE:    LoadScope   = LoadScope;
static CLONE:         Clone       = Clone;
static DROP:          Drop        = Drop;
static EQ:            Eq          = Eq;
static ASSERT:        Assert      = Assert;
static NOT:           Not         = Not;
static NULL:          Null        = Null;
static PTR_CAST:      PtrCast     = PtrCast;
static PTR_READ_CSTR: PtrReadCstr = PtrReadCstr;
static DEBUG_STACK:   DebugStack  = DebugStack;
static WHILE_DO:         WhileDo        = WhileDo;
static DO_WHILE:         DoWhile        = DoWhile;
static FN_TYPE:          FnType         = FnType;
static TYPED_ARGS:       TypedArgs      = TypedArgs;
static TYPED_RETS:       TypedRets      = TypedRets;
static UNTYPED_ARGS:     UntypedArgs    = UntypedArgs;
static UNTYPED_RETS:     UntypedRets    = UntypedRets;

/// Register the built-ins that are available at both runtime and comptime.
///
/// Runtime-only built-ins (`c_link_lib`, `ptr_read_cstr`) are NOT included here;
/// add them separately in `get_interpret_builtins`.
fn register_shared(m: &mut HashMap<&'static str, &'static dyn InterpretBuiltIn>) {
  m.insert("c_load_header", &C_LOAD_HEADER);
  m.insert("load_scope",    &LOAD_SCOPE);
  m.insert("clone",         &CLONE);
  m.insert("drop",          &DROP);
  m.insert("eq",            &EQ);
  m.insert("assert",        &ASSERT);
  m.insert("not",           &NOT);
  m.insert("null",          &NULL);
  m.insert("ptr_cast",        &PTR_CAST);
  m.insert("debug_stack",     &DEBUG_STACK);
  m.insert("fn",              &FN_TYPE);
  m.insert("typed_args",      &TYPED_ARGS);
  m.insert("typed_rets",      &TYPED_RETS);
  m.insert("untyped_args",    &UNTYPED_ARGS);
  m.insert("untyped_rets",    &UNTYPED_RETS);
}

pub fn get_interpret_builtins() -> HashMap<&'static str, &'static dyn InterpretBuiltIn> {
  let mut m = HashMap::new();
  register_shared(&mut m);
  m.insert("c_link_lib",    &C_LINK_LIB);
  m.insert("ptr_read_cstr", &PTR_READ_CSTR);
  m.insert("while_do",      &WHILE_DO);
  m.insert("do_while",      &DO_WHILE);
  m
}

/// The subset of interpret builtins available during the comptime pass.
pub fn get_comptime_builtins() -> HashMap<&'static str, &'static dyn InterpretBuiltIn> {
  let mut m = HashMap::new();
  register_shared(&mut m);
  m
}

/// Placeholder: compile builtins are registered here for use by the LLVM backend.
pub fn get_compile_builtins() -> HashMap<&'static str, &'static dyn CompileBuiltIn> {
  HashMap::new()
}
