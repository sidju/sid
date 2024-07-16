/// Defines the possible types at each stage of the execution process.
///
/// # Parsing -> TemplateValue:
/// Parsing returns a list of TemplateValue and the number of parent stack
/// entries needed to render it into a list of ProgramValue ready for execution.
///
/// # Rendering -> ProgramValue:
/// Parent stack is consumed and Parent label context read as needed to
/// Convert all TemplateValues into ProgramValues. When this is done the values
/// are ready to put on the program stack.
///
/// # Execution -> DataValue:
/// ProgramValue is a value ready to be executed, but not necessarily valid to
/// write onto the stack. Real and Label can be written directly, but Templates
/// are rendered into their concrete objects before writing them to the Stack
/// and Invoke tries to call the value on the top of the stack as a function.
///
///
/// # Extra confusion:
/// ProgramValue:s cannot be written to the data stack, but a Substack or
/// function can be written to the data stack even though it contains them.
///
/// As such, while a Template can't be written to the stack, a Substack
/// containing a Template can be handed around freely. This means that that
/// template is rendered when the Substack is invoked.
///
/// The allowed types are a structure of data lifecycle, not a restriction on
/// what is possible.

use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;

pub trait BuiltInFunction: Debug {
  fn execute(&self,
    data_stack: &mut Vec<DataValue>,
    program_queue: &mut VecDeque<ProgramValue>,
    local_scope: &mut HashMap<String, RealValue>,
    global_scope: &mut HashMap<String, RealValue>,
  );
}

#[derive(Debug, Clone, PartialEq)]
pub enum RealValue {
  Bool(bool),
  Str(String),
  Char(String), // Holds a full grapheme cluster, which requires a string
  Int(i64),
  Float(f64),
  Substack(Vec<ProgramValue>),
  List(Vec<DataValue>),
  BuiltInFunction(String),
}

#[derive(PartialEq, Clone, Debug)]
pub enum DataValue {
  Real(RealValue),
  Label(String),
}
impl  From<RealValue> for DataValue {
  fn from(item: RealValue) -> Self {
    Self::Real(item)
  }
}

// The values of the program as they look after parsing (before execution)
#[derive(PartialEq, Debug, Clone)]
pub enum ProgramValue{
  Real(RealValue),
  Label(String),
  Invoke,
  Template(Template),
}

#[derive(PartialEq, Debug, Clone)]
pub struct Template {
  pub data: TemplateData,
  pub consumes_stack_entries: usize,
}
impl  Template {
  pub fn substack(parsed: (Vec<TemplateValue>, usize)) -> Self {
    Self{
      data: TemplateData::SubstackTemplate(parsed.0),
      consumes_stack_entries: parsed.1,
    }
  }
  pub fn list(parsed: (Vec<TemplateValue>, usize)) -> Self {
    Self{
      data: TemplateData::ListTemplate(parsed.0),
      consumes_stack_entries: parsed.1,
    }
  }
}

#[derive(PartialEq, Debug, Clone)]
pub enum TemplateData {
  SubstackTemplate(Vec<TemplateValue>),
  ListTemplate(Vec<TemplateValue>),
//  ScriptTemplate,
//  StructTemplate,
//  ListTemplate,
//  SetTemplate,
}
impl  From<DataValue> for ProgramValue {
  fn from(item: DataValue) -> Self {
    match item {
      DataValue::Real(x) => Self::Real(x),
      DataValue::Label(l) => Self::Label(l),
    }
  }
}
impl  From<RealValue> for ProgramValue {
  fn from(item: RealValue) -> Self {
    Self::Real(item)
  }
}
impl  From<Template> for ProgramValue {
  fn from(item: Template) -> Self {
    Self::Template(item)
  }
}

#[derive(PartialEq, Debug, Clone)]
pub enum TemplateValue{
  ParentLabel(String),
  ParentStackMove(usize),
//  ParentStackCopy(usize), // Maybe?
  Literal(ProgramValue),
}
impl  From<DataValue> for TemplateValue {
  fn from(item: DataValue) -> Self {
    Self::Literal(item.into())
  }
}
impl  From<RealValue> for TemplateValue {
  fn from(item: RealValue) -> Self {
    Self::Literal(item.into())
  }
}
impl  From<ProgramValue> for TemplateValue {
  fn from(item: ProgramValue) -> Self {
    Self::Literal(item)
  }
}
impl  From<Template> for TemplateValue {
  fn from(item: Template) -> Self {
    Self::Literal(item.into())
  }
}
