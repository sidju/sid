use super::SideEffectFunction;

#[derive(PartialEq, Debug)]
pub enum Function {
  SideEffect(SideEffectFunction),
//  BuiltIn(BuiltInFunction),
}

#[derive(PartialEq, Debug)]
pub enum RealValue {
  Bool(bool),
  Str(String),
  Char(String), // Holds a full grapheme cluster, which requires a string
  Int(i64),
  Float(f64),
  Substack(Vec<ProgramValue>),
  Fun(Function),
}
impl From<SideEffectFunction> for RealValue {
  fn from(item: SideEffectFunction) -> Self {
    RealValue::Fun(Function::SideEffect(item))
  }
}

#[derive(PartialEq, Debug)]
pub enum Value {
  Real(RealValue),
  Label(String),
}
impl From<RealValue> for Value {
  fn from(item: RealValue) -> Self {
    Self::Real(item)
  }
}

// The values of the program as they look after parsing (before execution)
#[derive(PartialEq, Debug)]
pub enum ProgramValue{
  Real(RealValue),
  Label(String),
  Invoke,
  Template{
    consumed_stack_entries: usize,
    source: Template,
  },
}

#[derive(PartialEq, Debug)]
pub enum Template {
  SubstackTemplate(Vec<TemplateValue>),
//  ScriptTemplate,
//  StructTemplate,
//  ListTemplate,
//  SetTemplate,
}
impl From<Value> for ProgramValue {
  fn from(item: Value) -> Self {
    match item {
      Value::Real(x) => Self::Real(x),
      Value::Label(l) => Self::Label(l),
    }
  }
}
impl From<RealValue> for ProgramValue {
  fn from(item: RealValue) -> Self {
    Self::Real(item)
  }
}

#[derive(PartialEq, Debug)]
pub enum TemplateValue{
  ParentLabel(String),
  ParentStackMove(usize),
//  ParentStackCopy(usize), // Maybe?
  Literal(ProgramValue),
}
