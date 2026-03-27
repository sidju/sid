/// Defines the possible types at each stage of the execution process.
///
/// # Parsing -> TemplateValue:
/// Parsing returns a list of TemplateValue and the number of parent stack
/// entries needed to render it into a list of ProgramValue ready for execution.
///
/// # Rendering -> ProgramValue:
/// Parent stack is consumed and parent label context read as needed to
/// convert all TemplateValues into ProgramValues. When this is done the values
/// are ready to put on the program stack.
///
/// # Execution -> DataValue:
/// ProgramValues are popped from the program stack and acted on. `Data` pushes
/// its payload onto the data stack. `Template` renders into DataValues and
/// pushes them. `Invoke` pops the top of the data stack and executes it.
///
/// Labels are first-class values: they travel through the system as `DataValue::Label`
/// and are resolved lazily only when crossing a type boundary (e.g. on invoke
/// or when consumed by a typed function parameter). This mirrors the Erlang
/// property where atoms are valid values usable as implicit enums.

use std::collections::HashMap;
use std::ffi::CString;
use std::fmt::Debug;
use std::sync::Arc;
use anyhow::Result;
use libloading::Library;
use crate::type_system::SidType;
use crate::c_ffi::{CFunc, CFuncSig};

/// Global interpreter state: variables visible to the whole program plus a
/// registry of dynamically-loaded C shared libraries.
///
/// Splitting these two concerns into a single struct lets builtins like
/// `c_link_lib` load a library once and record it here, while `call_cfuncsig`
/// looks up (and lazily loads) the library by name at call time.
///
/// The scope is held as a mutable reference so callers retain ownership of
/// the underlying `HashMap` — no `mem::take`/put-back dance required.
pub struct GlobalState<'a> {
  /// Named values accessible from anywhere in the program.
  pub scope: &'a mut HashMap<String, DataValue>,
  /// Shared libraries loaded by `c_link_lib`, keyed by the path/soname used
  /// to load them.  Libraries are added on first use and reused thereafter.
  pub libraries: HashMap<String, Arc<Library>>,
}

impl<'a> GlobalState<'a> {
  pub fn new(scope: &'a mut HashMap<String, DataValue>) -> Self {
    Self { scope, libraries: HashMap::new() }
  }
}

/// Look up a label in scope, with support for dot-separated field access on structs.
///
/// `a.b.c` resolves `a` from scope, then walks `.b`, then `.c` as struct field
/// accesses. Any number of segments is supported.
///
/// Resolution order (highest to lowest priority):
/// 1. Local scope (if provided — absent at comptime).
/// 2. Global scope.
/// 3. Built-ins: any matching single-segment label returns `DataValue::BuiltIn`.
///    Pass `None` when no built-ins are relevant (e.g. pure render tests).
///
/// Returns an error if the root name is absent from all three, or if any
/// intermediate segment targets a non-struct or a missing field.
pub fn get_from_scope(
  label: &str,
  local: Option<&HashMap<String, DataValue>>,
  global: &HashMap<String, DataValue>,
  builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
) -> anyhow::Result<DataValue> {
  let mut segments = label.split('.');
  let root = segments.next().unwrap();

  let mut current = local.and_then(|l| l.get(root))
    .or_else(|| global.get(root))
    .cloned()
    .or_else(|| builtins.contains_key(root).then(|| DataValue::BuiltIn(root.to_owned())))
    .ok_or_else(|| anyhow::anyhow!("undefined label '{}'", label))?;

  for segment in segments {
    current = match current {
      DataValue::Map(entries) => entries
        .into_iter()
        .find(|(k, _)| matches!(k, DataValue::Label(n) if n == segment))
        .map(|(_, v)| v)
        .ok_or_else(|| anyhow::anyhow!("label '{}': field '{}' not found", label, segment))?,
      other => anyhow::bail!(
        "label '{}': expected a label-keyed Map for field '{}', got {:?}",
        label, segment, other
      ),
    };
  }
  Ok(current)
}

pub trait InterpretBuiltIn: Debug {
  fn execute(
    &self,
    data_stack: &mut Vec<TemplateValue>,
    global_state: &mut GlobalState<'_>,
    program_stack: &mut Vec<ProgramValue>,
    local_scope: &mut HashMap<String, DataValue>,
    builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
  ) -> Result<Vec<DataValue>>;
}

/// A built-in function that generates LLVM IR during compilation.
///
/// TODO: This trait is a placeholder. Methods for IR generation via the LLVM
/// backend will be added here when the compile path is further developed.
/// CompileBuiltIn functions can only be integration-tested against the LLVM
/// backend; they are not unit-testable with the interpreter alone.
pub trait CompileBuiltIn: Debug {
  fn arg_count(&self) -> u8;
  fn return_count(&self) -> u8;
}

/// A fully concrete value — the currency of the data stack and scope maps.
///
/// `Label` is included here because labels are first-class values that can be
/// passed around freely and resolved lazily at type boundaries.
///
/// `Substack` holds a packaged sequence of `ProgramValue`s. This is the
/// intentional exception to direct nesting: functions and closures are
/// represented as substacks, making them first-class citizens.
#[derive(Debug, Clone, PartialEq)]
pub enum DataValue {
  Bool(bool),
  /// A C-compatible string: null-terminated, no interior NUL bytes.
  /// Using `CString` rather than `String` removes the UTF-8 requirement and
  /// allows direct use as a `char *` in C FFI calls without re-allocation.
  Str(CString),
  Char(String), // Holds a full grapheme cluster, which requires a string
  Int(i64),
  Float(f64),
  /// A packaged program sequence — the representation of functions/closures.
  /// Holds `ProgramValue`s rather than `DataValue`s so that un-rendered
  /// templates inside a substack are rendered only when the substack is invoked.
  ///
  /// `args[0]`/`ret[0]` = **top** of stack; `[N-1]` = deepest checked item.
  ///
  /// `args` uses named fields: each entry is `(name, type)`.  At call time the
  /// types are checked and the names are bound into the callee's local scope.
  Substack { body: Vec<ProgramValue>, args: Option<Vec<(String, SidType)>>, ret: Option<Vec<SidType>> },
  /// Like `Substack` but sequential execution is guaranteed (no concurrency).
  ///
  /// `args[0]`/`ret[0]` = **top** of stack; `[N-1]` = deepest checked item.
  ///
  /// `args` uses named fields: each entry is `(name, type)`.  At call time the
  /// types are checked and the names are bound into the callee's local scope.
  Script { body: Vec<ProgramValue>, args: Option<Vec<(String, SidType)>>, ret: Option<Vec<SidType>> },
  List(Vec<DataValue>),
  Set(Vec<DataValue>),
  /// An ordered sequence of key-value pairs.  When all keys are
  /// `DataValue::Label`, this serves as a struct (named fields, dot-label
  /// access); when keys are arbitrary values, it is a true heterogeneous map.
  /// Both forms are ordered and structurally identical — the distinction is
  /// purely in the key type.
  Map(Vec<(DataValue, DataValue)>),
  BuiltIn(String),
  Type(SidType),
  Label(String),
  /// A dynamically-loaded C function, callable via libffi.
  CFunction(Arc<CFunc>),
  /// A raw C pointer returned from a C function call.
  /// The `pointee_ty` records the declared pointee type; `SidType::Any` for `void*`.
  Pointer { addr: usize, pointee_ty: SidType },
  /// A C function signature parsed from a header file.
  /// Stored in scope under the function's name by `c_load_header`.
  /// Replaced with `CFunction` when `c_link_lib` resolves it against a library.
  CFuncSig(CFuncSig),
  /// Interpreter-internal sentinel pushed onto the **data** stack to mark the
  /// boundary below a typed substack's arguments.  Acts as a hard floor: if any
  /// operation tries to use `StackBlock` as a real value the program panics.
  /// Removed by the corresponding `TypeCheck { block_placed: true }` sentinel
  /// after the substack body completes.
  StackBlock,
}

impl DataValue {
  /// Test whether `self`, used as a **pattern**, matches `value`.
  ///
  /// - If `self` is `DataValue::Type(t)`, delegates to `t.matches(value)`.
  /// - Otherwise wraps `self` in `SidType::Literal` and delegates, which
  ///   handles List-as-tuple, Set-as-enum, Map/Struct structural checks,
  ///   and exact equality for everything else.
  pub fn pattern_matches(&self, value: &DataValue) -> bool {
    match self {
      DataValue::Type(t) => t.matches(value),
      other => SidType::Literal(Box::new(other.clone())).matches(value),
    }
  }
}

/// A value on the program stack: either concrete data ready to push, a pending
/// invocation (`!`), a compile-time invocation (`@!`), an unrendered template,
/// or a while-loop checkpoint.
#[derive(PartialEq, Debug, Clone)]
pub enum ProgramValue {
  Data(DataValue),
  Invoke,
  ComptimeInvoke,
  Template(Template),
  /// Sentinel that asserts the data stack has exactly `expected_len` items when
  /// popped. Panics with `message` if not. Useful anywhere a stack-size contract
  /// must be enforced at a specific point — e.g. after a loop body to catch
  /// net-non-zero bodies before the condition runs on a corrupt stack.
  StackSizeAssert { expected_len: usize, message: &'static str },
  /// Sentinel placed on the program stack after a `while_do`/`do_while` condition runs.
  /// When popped it validates that the condition left exactly one `Bool` on top
  /// (stack depth must equal `expected_len + 1`), then either re-queues
  /// `body → cond → CondLoop` (true) or exits (false).
  CondLoop {
    cond: Vec<ProgramValue>,
    body: Vec<ProgramValue>,
    /// Stack depth recorded immediately after popping the condition and body
    /// substacks — i.e. the "loop state" size. The condition must leave this
    /// many items plus exactly one `Bool` on top; the body must leave exactly
    /// this many items (net zero change).
    expected_len: usize,
  },
  /// Sentinel placed on the program stack to validate the data stack (and
  /// optionally clean up a `StackBlock`) when popped.
  ///
  /// - `types`: if `Some`, the types that must match the return window.
  ///   `types[0]` = top of window, `types[N-1]` = deepest.
  ///   If `None`, no type checking is performed — the sentinel only cleans up
  ///   the `StackBlock`.
  /// - `context`: included in any panic message to identify the call site.
  /// - `block_placed`: if `true`, find and remove the nearest `StackBlock`
  ///   from the data stack.  When `types` is also `Some`, every item above the
  ///   block is checked against the declared types (strict count match).
  ///   If `false`, `types` is checked against the top `types.len()` items of
  ///   the full stack (legacy behaviour, used when only `ret` is declared).
  TypeCheck { types: Option<Vec<SidType>>, context: String, block_placed: bool },
  /// Saves the current local scope onto the scope stack and installs a fresh
  /// empty scope.  Paired with `PopScope`.  Every substack body is wrapped in
  /// `PushScope` / `PopScope` to isolate its local bindings.
  ///
  /// `names` holds the field names for the callee's declared `args` (top-first).
  /// When non-empty, `PushScope` pops that many items from the data stack and
  /// binds them into the new local scope under the corresponding names before
  /// the body begins.  Empty when the callee has no named args.
  PushScope { names: Vec<String> },
  /// Restores the local scope saved by the matching `PushScope`.
  PopScope,
}

impl From<DataValue> for ProgramValue {
  fn from(item: DataValue) -> Self { Self::Data(item) }
}
impl From<Template> for ProgramValue {
  fn from(item: Template) -> Self { Self::Template(item) }
}

#[derive(PartialEq, Debug, Clone)]
pub struct Template {
  pub data: TemplateData,
  pub consumes_stack_entries: usize,
  /// If true, this template is rendered eagerly during the comptime pass.
  pub comptime: bool,
}
impl Template {
  pub fn substack(parsed: (Vec<TemplateValue>, usize)) -> Self {
    Self { data: TemplateData::Substack(parsed.0), consumes_stack_entries: parsed.1, comptime: false }
  }
  pub fn list(parsed: (Vec<TemplateValue>, usize)) -> Self {
    Self { data: TemplateData::List(parsed.0), consumes_stack_entries: parsed.1, comptime: false }
  }
  pub fn set(parsed: (Vec<TemplateValue>, usize)) -> Self {
    Self { data: TemplateData::Set(parsed.0), consumes_stack_entries: parsed.1, comptime: false }
  }
  pub fn map(pairs: Vec<(TemplateValue, TemplateValue)>, consumes: usize) -> Self {
    Self { data: TemplateData::Map(pairs), consumes_stack_entries: consumes, comptime: false }
  }
  pub fn script(parsed: (Vec<TemplateValue>, usize)) -> Self {
    Self { data: TemplateData::Script(parsed.0), consumes_stack_entries: parsed.1, comptime: false }
  }
  /// Mark this template for eager evaluation during the comptime pass.
  pub fn mark_comptime(mut self) -> Self {
    self.comptime = true;
    self
  }
}

#[derive(PartialEq, Debug, Clone)]
pub enum TemplateData {
  Substack(Vec<TemplateValue>),
  List(Vec<TemplateValue>),
  Script(Vec<TemplateValue>),
  Set(Vec<TemplateValue>),
  /// `{key: value, …}` — keys are any TemplateValue (labels treated as values).
  /// Produces a `DataValue::Map`; use `struct!` to further narrow to a Struct.
  Map(Vec<(TemplateValue, TemplateValue)>),
}

#[derive(PartialEq, Debug, Clone)]
pub enum TemplateValue {
  ParentLabel(String),
  ParentStackMove(usize),
//  ParentStackCopy(usize), // Maybe?
  Literal(ProgramValue),
}
impl From<DataValue> for TemplateValue {
  fn from(item: DataValue) -> Self { Self::Literal(item.into()) }
}
impl From<ProgramValue> for TemplateValue {
  fn from(item: ProgramValue) -> Self { Self::Literal(item) }
}
impl From<Template> for TemplateValue {
  fn from(item: Template) -> Self { Self::Literal(item.into()) }
}
