use std::collections::HashMap;
use std::ffi::CString;

use crate::InterpretBuiltIn;
use crate::CompileBuiltIn;
use crate::DataValue;
use crate::GlobalState;
use crate::SidType;
use crate::c_ffi::{parse_c_header, open_library};
use crate::types::{get_from_scope, resolve_if_label};

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

/// Pop the top concrete `DataValue` from the data stack, automatically
/// resolving any `Label` to the value it points to in scope.
///
/// Use this instead of `pop_arg` in built-ins that expect a concrete typed
/// value (Bool, Int, Substack, Map, …) — it lets callers pass a bare label
/// wherever the direct value would be accepted.
fn pop_arg_resolved(
  data_stack: &mut Vec<crate::TemplateValue>,
  builtin_name: &str,
  local_scope: &HashMap<String, DataValue>,
  global_scope: &HashMap<String, DataValue>,
  builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
) -> anyhow::Result<DataValue> {
  let v = pop_arg(data_stack, builtin_name)?;
  Ok(resolve_if_label(v, Some(local_scope), Some(global_scope), Some(builtins)))
}

/// Pop the top concrete `DataValue` from the data stack, expecting a `Label`.
/// Returns the label string, or an error if the value is not a label.
fn pop_label(
  data_stack: &mut Vec<crate::TemplateValue>,
  builtin_name: &str,
) -> anyhow::Result<String> {
  match pop_arg(data_stack, builtin_name)? {
    DataValue::Label(l) => Ok(l),
    other => anyhow::bail!("{}: expected a label, got {:?}", builtin_name, other),
  }
}


/// Returns the default global scope, pre-populated with a `types` struct
/// containing the primitive type values (`types.bool`, `types.int`, etc.) and
/// the null pointer constant (`types.null`).
///
/// Built-in function names are **not** stored here; they are resolved as the
/// lowest-priority fallback inside `get_from_scope` by passing the relevant
/// `builtin_names` slice at each call site.
pub fn default_scope() -> HashMap<String, DataValue> {
  let types = DataValue::Map(vec![
    (DataValue::Label("bool".to_owned()),  DataValue::Type(SidType::Bool)),
    (DataValue::Label("int".to_owned()),   DataValue::Type(SidType::Int)),
    (DataValue::Label("float".to_owned()), DataValue::Type(SidType::Float)),
    (DataValue::Label("char".to_owned()),  DataValue::Type(SidType::Char)),
    (DataValue::Label("str".to_owned()),   DataValue::Type(SidType::Str)),
    (DataValue::Label("label".to_owned()), DataValue::Type(SidType::Label)),
    (DataValue::Label("any".to_owned()),   DataValue::Type(SidType::Any)),
    (DataValue::Label("value".to_owned()), DataValue::Type(SidType::Value)),
    (DataValue::Label("null".to_owned()),  DataValue::Pointer { addr: 0, pointee_ty: SidType::Any }),
  ]);
  let mut m = HashMap::new();
  m.insert("types".to_owned(), types);
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
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let arg = pop_arg_resolved(data_stack, "c_load_header", local_scope, global_state.scope, builtins)?;
    let (header_path, lib_name) = parse_load_header_arg(arg)?;
    let sigs = parse_c_header(&header_path, &lib_name)?;
    let out_fields: Vec<(DataValue, DataValue)> = sigs
      .into_iter()
      .map(|s| {
        let name = s.name.clone();
        (DataValue::Label(name), DataValue::CFuncSig(s))
      })
      .collect();
    Ok(vec![DataValue::Map(out_fields)])
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
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let arg = pop_arg_resolved(data_stack, "c_link_lib", local_scope, global_state.scope, builtins)?;
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
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let entries = match pop_arg_resolved(data_stack, "load_scope", local_scope, global_state.scope, builtins)? {
      DataValue::Map(e) => e,
      other => anyhow::bail!("load_scope expects a label-keyed Map, got {:?}", other),
    };
    for (key, value) in entries {
      match key {
        DataValue::Label(name) => { global_state.scope.insert(name, value); },
        other => anyhow::bail!("load_scope: key must be a Label, got {:?}", other),
      }
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
    _local_scope: &mut HashMap<String, DataValue>,
    _builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
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
    _local_scope: &mut HashMap<String, DataValue>,
    _builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
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
    _local_scope: &mut HashMap<String, DataValue>,
    _builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
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
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    match pop_arg_resolved(data_stack, "assert", local_scope, global_state.scope, builtins)? {
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
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    match pop_arg_resolved(data_stack, "not", local_scope, global_state.scope, builtins)? {
      DataValue::Bool(b) => Ok(vec![DataValue::Bool(!b)]),
      other => anyhow::bail!("not expects Bool, got {:?}", other),
    }
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
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let new_type = pop_arg(data_stack, "ptr_cast")?;
    let pointer  = pop_arg(data_stack, "ptr_cast")?;
    let addr = match pointer {
      DataValue::Pointer { addr, .. } => addr,
      other => anyhow::bail!("ptr_cast: first argument must be a Pointer, got {:?}", other),
    };
    let pointee_ty = match new_type {
      DataValue::Type(ty) => ty,
      DataValue::Label(name) => match get_from_scope(&name, Some(local_scope), Some(global_state.scope), Some(builtins))? {
        DataValue::Type(ty) => ty,
        other => anyhow::bail!("ptr_cast: label '{}' resolves to {:?}, not a Type", name, other),
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
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    match pop_arg_resolved(data_stack, "ptr_read_cstr", local_scope, global_state.scope, builtins)? {
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
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let n = match pop_arg_resolved(data_stack, "debug_stack", local_scope, global_state.scope, builtins)? {
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
/// The initial condition run is unconstrained — it may consume or produce any
/// number of stack values as long as it leaves exactly one `Bool` on top.
/// After that Bool is popped, `expected_len` is captured from the live stack
/// and all subsequent iterations must satisfy the combined invariant:
/// body + condition together are net +1 (leaving exactly one `Bool`).
///
/// Stack contract:
///   - Initial condition: leaves one `Bool` on top (stack may otherwise change).
///   - Each subsequent body+condition pair: net +1 `Bool` relative to `expected_len`.
#[derive(Debug)]
struct WhileDo;

impl InterpretBuiltIn for WhileDo {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    global_state: &mut GlobalState<'_>,
    program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let body_val = pop_arg_resolved(data_stack, "while_do", local_scope, global_state.scope, builtins)?;
    let cond_val = pop_arg_resolved(data_stack, "while_do", local_scope, global_state.scope, builtins)?;
    match &body_val { DataValue::Substack { .. } => {}, other => anyhow::bail!("while_do: body must be a Substack, got {:?}", other) }
    match &cond_val { DataValue::Substack { .. } => {}, other => anyhow::bail!("while_do: condition must be a Substack, got {:?}", other) }
    // Schedule: cond runs first (as a proper invocation), then CondLoopStart captures expected_len.
    program_stack.push(crate::ProgramValue::CondLoopStart { cond: cond_val.clone(), body: body_val });
    program_stack.push(crate::ProgramValue::Invoke);
    program_stack.push(crate::ProgramValue::Data(cond_val));
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
/// once. `expected_len` is captured before the first body run; all subsequent
/// body+condition pairs must satisfy the combined invariant: net +1 `Bool`.
///
/// Stack contract:
///   - Each body+condition pair: net +1 `Bool` relative to `expected_len`
///     (body may leave values for the condition to consume).
#[derive(Debug)]
struct DoWhile;

impl InterpretBuiltIn for DoWhile {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    global_state: &mut GlobalState<'_>,
    program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let cond_val = pop_arg_resolved(data_stack, "do_while", local_scope, global_state.scope, builtins)?;
    let body_val = pop_arg_resolved(data_stack, "do_while", local_scope, global_state.scope, builtins)?;
    match &cond_val { DataValue::Substack { .. } => {}, other => anyhow::bail!("do_while: condition must be a Substack, got {:?}", other) }
    match &body_val { DataValue::Substack { .. } => {}, other => anyhow::bail!("do_while: body must be a Substack, got {:?}", other) }
    let expected_len = data_stack.len();
    // Schedule: body runs first (as a proper invocation), then cond, then CondLoop.
    program_stack.push(crate::ProgramValue::CondLoop { cond: cond_val.clone(), body: body_val.clone(), expected_len });
    program_stack.push(crate::ProgramValue::Invoke);
    program_stack.push(crate::ProgramValue::Data(cond_val));
    program_stack.push(crate::ProgramValue::Invoke);
    program_stack.push(crate::ProgramValue::Data(body_val));
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
    _local_scope: &mut HashMap<String, DataValue>,
    _builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    Ok(vec![DataValue::Type(SidType::Fn { args: None, ret: None })])
  }
}

/// Pops a type value and wraps it in `SidType::Pointer`.
///
/// Usage: `T ptr !` → `T ptr` type
#[derive(Debug)]
struct PtrType;

impl InterpretBuiltIn for PtrType {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let raw = pop_arg(data_stack, "ptr")?;
    let resolved = match raw {
      DataValue::Label(ref l) => get_from_scope(l, Some(local_scope), Some(global_state.scope), Some(builtins))
        .map_err(|_| anyhow::anyhow!("ptr: undefined label '{}'", l))?,
      other => other,
    };
    let inner = match resolved {
      DataValue::Type(t) => t,
      other => anyhow::bail!("ptr: expected a type, got {:?}", other),
    };
    Ok(vec![DataValue::Type(SidType::Pointer(Box::new(inner)))])
  }
}

/// Pops a type value and wraps it in `SidType::List`.
///
/// Usage: `T list !` → `T list` type (homogeneous list of T)
#[derive(Debug)]
struct ListType;

impl InterpretBuiltIn for ListType {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let raw = pop_arg(data_stack, "list")?;
    let resolved = match raw {
      DataValue::Label(ref l) => get_from_scope(l, Some(local_scope), Some(global_state.scope), Some(builtins))
        .map_err(|_| anyhow::anyhow!("list: undefined label '{}'", l))?,
      other => other,
    };
    let inner = match resolved {
      DataValue::Type(t) => t,
      other => anyhow::bail!("list: expected a type, got {:?}", other),
    };
    Ok(vec![DataValue::Type(SidType::List(Box::new(inner)))])
  }
}

/// Pops two type values and returns a `Require` type: value must match both.
///
/// Usage: `base constraint require @!` → combined type
///
/// Arguments may be `DataValue::Type` (for type checks) or any other
/// `DataValue` (wrapped as `SidType::Literal` for exact-value matching).
#[derive(Debug)]
struct RequireType;

impl InterpretBuiltIn for RequireType {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    _local_scope: &mut HashMap<String, DataValue>,
    _builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let resolve_type = |raw: DataValue| -> anyhow::Result<SidType> {
      match raw {
        DataValue::Type(t) => Ok(t),
        // Accept any plain value (including labels) as an exact-match constraint.
        other => Ok(SidType::Literal(Box::new(other))),
      }
    };
    let constraint = resolve_type(pop_arg(data_stack, "require")?)?;
    let base       = resolve_type(pop_arg(data_stack, "require")?)?;
    Ok(vec![DataValue::Type(SidType::Require {
      base:       Box::new(base),
      constraint: Box::new(constraint),
    })])
  }
}

/// Pops two type values and returns an `Exclude` type: value must match base
/// but must NOT match forbidden.
///
/// Usage: `base forbidden exclude @!` → combined type
#[derive(Debug)]
struct ExcludeType;

impl InterpretBuiltIn for ExcludeType {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    _local_scope: &mut HashMap<String, DataValue>,
    _builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let resolve_type = |raw: DataValue| -> anyhow::Result<SidType> {
      match raw {
        DataValue::Type(t) => Ok(t),
        // Accept any plain value (including labels) as an exact-match exclusion.
        other => Ok(SidType::Literal(Box::new(other))),
      }
    };
    let forbidden = resolve_type(pop_arg(data_stack, "exclude")?)?;
    let base      = resolve_type(pop_arg(data_stack, "exclude")?)?;
    Ok(vec![DataValue::Type(SidType::Exclude {
      base:     Box::new(base),
      forbidden: Box::new(forbidden),
    })])
  }
}


///
/// Usage: `callable {name: T, …} typed_args !`
///
/// The map keys must all be labels and values must all be types.  The map is
/// given deepest-first (first field = deepest/last-pushed arg); it is stored
/// reversed (top-first) to match the `args[0]`=top convention.
///
/// At call time the arg types are verified and each arg value is consumed from
/// the data stack and bound into the callee's fresh local scope under its name.
#[derive(Debug)]
struct TypedArgs;

impl InterpretBuiltIn for TypedArgs {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let target_val = pop_arg_resolved(data_stack, "typed_args", local_scope, global_state.scope, builtins)?;
    let types_val  = pop_arg_resolved(data_stack, "typed_args", local_scope, global_state.scope, builtins)?;

    // Validate shape: must be a Map with all label keys.
    let label_map_ty = SidType::Map {
      key: Box::new(SidType::Label),
      value: Box::new(SidType::Any),
    };
    if !label_map_ty.matches(&types_val) {
      anyhow::bail!("typed_args: expected a label-keyed Map, got {:?}", types_val);
    }
    let entries = match types_val { DataValue::Map(e) => e, _ => unreachable!() };

    // Extract (name, type) pairs; written deepest-first → stored top-first.
    let named_args: Vec<(String, SidType)> = entries.into_iter().rev().map(|(k, v)| {
      let name = match k { DataValue::Label(s) => s, _ => unreachable!() };
      let ty = match v {
        DataValue::Type(t) => t,
        other => anyhow::bail!(
          "typed_args: field '{}' value must be a type, got {:?}", name, other
        ),
      };
      Ok((name, ty))
    }).collect::<anyhow::Result<_>>()?;

    // For the abstract Fn type, store type-only (no names at type level).
    let type_only: Vec<SidType> = named_args.iter().map(|(_, t)| t.clone()).collect();
    match target_val {
      DataValue::Substack { body, ret, .. } =>
        Ok(vec![DataValue::Substack { body, args: Some(named_args), ret }]),
      DataValue::Script { body, ret, .. } =>
        Ok(vec![DataValue::Script { body, args: Some(named_args), ret }]),
      DataValue::Type(SidType::Fn { ret, .. }) =>
        Ok(vec![DataValue::Type(SidType::Fn { args: Some(type_only), ret })]),
      other => anyhow::bail!("typed_args: expected Substack, Script, or Fn type, got {:?}", other),
    }
  }
}

/// Sets the `ret` type annotation on a `Substack`/`Script` or `SidType::Fn`.
///
/// Usage: `callable [T1 T2 …] typed_rets !`
/// The list is given deepest-first; it is stored reversed (top-first).
#[derive(Debug)]
struct TypedRets;

impl InterpretBuiltIn for TypedRets {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let target_val = pop_arg_resolved(data_stack, "typed_rets", local_scope, global_state.scope, builtins)?;
    let types_val  = pop_arg_resolved(data_stack, "typed_rets", local_scope, global_state.scope, builtins)?;
    // Reverse: user writes deepest-first, stored top-first.
    let types: Vec<SidType> = list_to_type_vec(types_val, "typed_rets")?.into_iter().rev().collect();
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
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let target_val = pop_arg_resolved(data_stack, "untyped_args", local_scope, global_state.scope, builtins)?;
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
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let target_val = pop_arg_resolved(data_stack, "untyped_rets", local_scope, global_state.scope, builtins)?;
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

// ── local ─────────────────────────────────────────────────────────────────────

/// Built-in that binds a value to a name in the **local** scope.
///
/// Pops two values: first `value` (any), then `name` (a label).
/// Identical to `def` except it writes to local scope instead of global scope.
///
/// Available at **both** comptime and runtime.
#[derive(Debug)]
struct Local;

impl InterpretBuiltIn for Local {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    _builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let value = pop_arg(data_stack, "local")?;
    let name = match pop_arg(data_stack, "local")? {
      DataValue::Label(l) => l,
      other => anyhow::bail!("local: name must be a label, got {:?}", other),
    };
    local_scope.insert(name, value);
    Ok(vec![])
  }
}

// ── load_local ────────────────────────────────────────────────────────────────

/// Built-in that unpacks a label-keyed `DataValue::Map` into the **local** scope.
///
/// Argument: `DataValue::Map` where every key is a `DataValue::Label`.
/// Return:   nothing.
///
/// Identical to `load_scope` except it writes to local scope.
/// Available at **both** comptime and runtime.
#[derive(Debug)]
struct LoadLocal;

impl InterpretBuiltIn for LoadLocal {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let entries = match pop_arg_resolved(data_stack, "load_local", local_scope, global_state.scope, builtins)? {
      DataValue::Map(e) => e,
      other => anyhow::bail!("load_local expects a label-keyed Map, got {:?}", other),
    };
    for (key, value) in entries {
      match key {
        DataValue::Label(name) => { local_scope.insert(name, value); },
        other => anyhow::bail!("load_local: key must be a Label, got {:?}", other),
      }
    }
    Ok(vec![])
  }
}

// ── match ─────────────────────────────────────────────────────────────────────

/// Built-in that dispatches a value against an ordered map of pattern/action
/// cases, invoking the first matching action.
///
/// Usage: `value {pattern: action, ...} match !`
///
/// - `cases` must be a `DataValue::Map`, or a label that resolves to one.
///   The map is iterated in declaration order (first-match-wins).
/// - Map keys are used directly as patterns via `DataValue::pattern_matches`.
///   `Type` values delegate to their `matches` method; all other values require
///   exact equality.  Use `$types.int`, `$types.any`, etc. for type dispatch;
///   use bare label keys (which render as `DataValue::Label`) for enum dispatch.
/// - `action` must be a `DataValue::Substack` or `Script`; its body is pushed
///   onto the program stack for execution.
/// - The matched value is consumed.
/// - All branches must leave the stack with the same net size change — a
///   programmer contract, not enforced at runtime since only one branch runs.
/// - Panics if no case matches.
///
/// Runtime-only (manipulates the program stack).
#[derive(Debug)]
struct Match;

impl InterpretBuiltIn for Match {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    global_state: &mut GlobalState<'_>,
    program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let cases_raw = pop_arg(data_stack, "match")?;
    // Resolve a label to its value, supporting dot-notation namespaces.
    let cases_val = match cases_raw {
      DataValue::Label(ref l) => get_from_scope(l, Some(local_scope), Some(global_state.scope), Some(builtins))
        .map_err(|_| anyhow::anyhow!("match: undefined label '{}'", l))?,
      other => other,
    };
    let value = pop_arg(data_stack, "match")?;

    let entries = match cases_val {
      DataValue::Map(e) => e,
      other => anyhow::bail!("match: cases must be a Map, got {:?}", other),
    };

    for (pattern, action) in entries {
      if pattern.pattern_matches(&value) {
        let body = match action {
          DataValue::Substack { body, .. } | DataValue::Script { body, .. } => body,
          other => anyhow::bail!("match: action must be a Substack or Script, got {:?}", other),
        };
        program_stack.extend(body.into_iter().rev());
        return Ok(vec![]);
      }
    }

    anyhow::bail!("match: no case matched value {:?}", value)
  }
}

// ── scope lookup built-ins ────────────────────────────────────────────────────

/// Pops a label and looks it up following the normal local → global → builtins
/// priority order.  At comptime the local scope is empty, so this effectively
/// falls through to global scope.
#[derive(Debug)]
struct Get;

impl InterpretBuiltIn for Get {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let label = pop_label(data_stack, "get")?;
    let value = get_from_scope(&label, Some(local_scope), Some(global_state.scope), Some(builtins))
      .map_err(|_| anyhow::anyhow!("get: '{}' not found", label))?;
    Ok(vec![value])
  }
}

/// Pops a label and looks it up in local scope only.  Errors if not found.
/// At comptime the local scope is always empty, so this will always error.
#[derive(Debug)]
struct GetLocal;

impl InterpretBuiltIn for GetLocal {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    _global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    _builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let label = pop_label(data_stack, "get_local")?;
    let value = get_from_scope(&label, Some(local_scope), None, None)
      .map_err(|_| anyhow::anyhow!("get_local: '{}' not found in local scope", label))?;
    Ok(vec![value])
  }
}

/// Pops a label and looks it up in global scope only, bypassing local scope.
/// Useful at comptime to access global definitions (e.g. `types.int get_global @!`),
/// and at runtime to access a global that is shadowed by a local binding.
#[derive(Debug)]
struct GetGlobal;

impl InterpretBuiltIn for GetGlobal {
  fn execute(
    &self,
    data_stack: &mut Vec<crate::TemplateValue>,
    global_state: &mut GlobalState<'_>,
    _program_stack: &mut Vec<crate::ProgramValue>,
    _local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> anyhow::Result<Vec<DataValue>> {
    let label = pop_label(data_stack, "get_global")?;
    let value = get_from_scope(&label, None, Some(global_state.scope), Some(builtins))
      .map_err(|_| anyhow::anyhow!("get_global: '{}' not found in global scope", label))?;
    Ok(vec![value])
  }
}

// Module-level statics so both get_interpret_builtins and get_comptime_builtins
// can reference them without duplicating declarations.
static GET:           Get         = Get;
static GET_LOCAL:     GetLocal    = GetLocal;
static GET_GLOBAL:    GetGlobal   = GetGlobal;
static LOCAL:         Local       = Local;
static LOAD_LOCAL:    LoadLocal   = LoadLocal;
static C_LOAD_HEADER: CLoadHeader = CLoadHeader;
static C_LINK_LIB:    CLinkLib    = CLinkLib;
static LOAD_SCOPE:    LoadScope   = LoadScope;
static CLONE:         Clone       = Clone;
static DROP:          Drop        = Drop;
static EQ:            Eq          = Eq;
static ASSERT:        Assert      = Assert;
static NOT:           Not         = Not;
static PTR_CAST:      PtrCast     = PtrCast;
static PTR_READ_CSTR: PtrReadCstr = PtrReadCstr;
static DEBUG_STACK:   DebugStack  = DebugStack;
static WHILE_DO:         WhileDo        = WhileDo;
static DO_WHILE:         DoWhile        = DoWhile;
static FN_TYPE:          FnType         = FnType;
static PTR_TYPE:         PtrType        = PtrType;
static LIST_TYPE:        ListType       = ListType;
static REQUIRE_TYPE:     RequireType    = RequireType;
static EXCLUDE_TYPE:     ExcludeType    = ExcludeType;
static TYPED_ARGS:       TypedArgs      = TypedArgs;
static TYPED_RETS:       TypedRets      = TypedRets;
static UNTYPED_ARGS:     UntypedArgs    = UntypedArgs;
static UNTYPED_RETS:     UntypedRets    = UntypedRets;
static MATCH:            Match          = Match;

/// Register the built-ins that are available at both runtime and comptime.
///
/// Runtime-only built-ins (`c_link_lib`, `ptr_read_cstr`) are NOT included here;
/// add them separately in `get_interpret_builtins`.
fn register_shared(m: &mut HashMap<&'static str, &'static dyn InterpretBuiltIn>) {
  m.insert("get",          &GET);
  m.insert("get_local",    &GET_LOCAL);
  m.insert("get_global",   &GET_GLOBAL);
  m.insert("c_load_header", &C_LOAD_HEADER);
  m.insert("load_scope",    &LOAD_SCOPE);
  m.insert("local",         &LOCAL);
  m.insert("load_local",    &LOAD_LOCAL);
  m.insert("clone",         &CLONE);
  m.insert("drop",          &DROP);
  m.insert("eq",            &EQ);
  m.insert("assert",        &ASSERT);
  m.insert("not",           &NOT);
  m.insert("ptr_cast",        &PTR_CAST);
  m.insert("debug_stack",     &DEBUG_STACK);
  m.insert("fn",              &FN_TYPE);
  m.insert("ptr",             &PTR_TYPE);
  m.insert("list",            &LIST_TYPE);
  m.insert("require",         &REQUIRE_TYPE);
  m.insert("exclude",         &EXCLUDE_TYPE);
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
  m.insert("match",         &MATCH);
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
