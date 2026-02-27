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
use std::fmt::Debug;
use anyhow::Result;
use crate::type_system::SidType;

pub trait InterpretBuiltIn: Debug {
  /// Number of arguments popped from the data stack (0 or 1).
  fn arg_count(&self) -> u8;
  /// Number of values pushed back onto the data stack (0 or 1).
  fn return_count(&self) -> u8;
  fn execute(
    &self,
    arg: Option<DataValue>,
    global_scope: &HashMap<String, DataValue>,
  ) -> Result<Option<DataValue>>;
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
  Str(String),
  Char(String), // Holds a full grapheme cluster, which requires a string
  Int(i64),
  Float(f64),
  /// A packaged program sequence — the representation of functions/closures.
  /// Holds `ProgramValue`s rather than `DataValue`s so that un-rendered
  /// templates inside a substack are rendered only when the substack is invoked.
  Substack(Vec<ProgramValue>),
  /// Like `Substack` but sequential execution is guaranteed (no concurrency).
  Script(Vec<ProgramValue>),
  List(Vec<DataValue>),
  Set(Vec<DataValue>),
  Struct(Vec<(String, DataValue)>),
  Map(Vec<(DataValue, DataValue)>),
  BuiltIn(String),
  Type(SidType),
  Label(String),
}

/// A value on the program stack: either concrete data ready to push, a pending
/// invocation (`!`), a compile-time invocation (`@!`), or an unrendered
/// template containing substitution slots.
#[derive(PartialEq, Debug, Clone)]
pub enum ProgramValue {
  Data(DataValue),
  Invoke,
  ComptimeInvoke,
  Template(Template),
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
