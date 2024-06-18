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

use std::collections::HashMap;
use std::fmt::Debug;

pub trait BuiltInFunction: Debug {
  fn execute<'a>(&self,
    data_stack: &mut Vec<DataValue<'a>>,
    program_stack: &mut Vec<ProgramValue<'a>>,
    local_scope: &mut HashMap<String, RealValue<'a>>,
    global_scope: &mut HashMap<String, RealValue<'a>>,
  );
}

pub trait BuiltInFunctionWithPartialEquals: BuiltInFunction + PartialEq {}

#[derive(Debug, Clone)]
pub enum RealValue<'a> {
  Bool(bool),
  Str(String),
  Char(String), // Holds a full grapheme cluster, which requires a string
  Int(i64),
  Float(f64),
  Substack(Vec<ProgramValue<'a>>),
  BuiltInFunction(&'a dyn BuiltInFunction),
}
// Manual implementation, since we cannot compare dyn BuiltInFunction without
// knowing its type and thus need to always return false for that
impl <'a> PartialEq for RealValue<'a> {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Bool(x), Self::Bool(y)) if x == y => true,
      (Self::Str(x), Self::Str(y)) if x == y => true,
      (Self::Char(x), Self::Char(y)) if x == y => true,
      (Self::Int(x), Self::Int(y)) if x == y => true,
      (Self::Float(x), Self::Float(y)) if x == y => true,
      (Self::Substack(x), Self::Substack(y)) if x == y => true,
      _ => false 
    }
  }
}

#[derive(PartialEq, Debug)]
pub enum DataValue<'a> {
  Real(RealValue<'a>),
  Label(String),
}
impl <'a> From<RealValue<'a>> for DataValue<'a> {
  fn from(item: RealValue<'a>) -> Self {
    Self::Real(item)
  }
}

// The values of the program as they look after parsing (before execution)
#[derive(PartialEq, Debug, Clone)]
pub enum ProgramValue<'a>{
  Real(RealValue<'a>),
  Label(String),
  Invoke,
  Template(Template<'a>),
}

#[derive(PartialEq, Debug, Clone)]
pub struct Template<'a> {
  pub data: TemplateData<'a>,
  pub consumes_stack_entries: usize,
}
impl <'a> Template<'a> {
  pub fn substack(parsed: (Vec<TemplateValue<'a>>, usize)) -> Self {
    Self{
      data: TemplateData::SubstackTemplate(parsed.0),
      consumes_stack_entries: parsed.1,
    }
  }
}

#[derive(PartialEq, Debug, Clone)]
pub enum TemplateData<'a> {
  SubstackTemplate(Vec<TemplateValue<'a>>),
//  ScriptTemplate,
//  StructTemplate,
//  ListTemplate,
//  SetTemplate,
}
impl <'a> From<DataValue<'a>> for ProgramValue<'a> {
  fn from(item: DataValue<'a>) -> Self {
    match item {
      DataValue::Real(x) => Self::Real(x),
      DataValue::Label(l) => Self::Label(l),
    }
  }
}
impl <'a> From<RealValue<'a>> for ProgramValue<'a> {
  fn from(item: RealValue<'a>) -> Self {
    Self::Real(item)
  }
}
impl <'a> From<Template<'a>> for ProgramValue<'a> {
  fn from(item: Template<'a>) -> Self {
    Self::Template(item)
  }
}

#[derive(PartialEq, Debug, Clone)]
pub enum TemplateValue<'a>{
  ParentLabel(String),
  ParentStackMove(usize),
//  ParentStackCopy(usize), // Maybe?
  Literal(ProgramValue<'a>),
}
impl <'a> From<DataValue<'a>> for TemplateValue<'a> {
  fn from(item: DataValue<'a>) -> Self {
    Self::Literal(item.into())
  }
}
impl <'a> From<RealValue<'a>> for TemplateValue<'a> {
  fn from(item: RealValue<'a>) -> Self {
    Self::Literal(item.into())
  }
}
impl <'a> From<ProgramValue<'a>> for TemplateValue<'a> {
  fn from(item: ProgramValue<'a>) -> Self {
    Self::Literal(item)
  }
}
impl <'a> From<Template<'a>> for TemplateValue<'a> {
  fn from(item: Template<'a>) -> Self {
    Self::Literal(item.into())
  }
}
