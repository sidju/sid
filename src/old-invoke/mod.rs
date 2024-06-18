
use super::{
  Value,
  RealValue,
  Function,
};

pub mod side_effects;
use side_effects::SideEffectFunction;

pub fn resolve_label(
  label: &str,
) -> RealValue {
  // First we match against built in functions
  match label {
    "print" => SideEffectFunction::Print.into(),
    _ => panic!("Label not found"),
  }
}

pub fn realize_value(
  possibly_label: Value,
) -> RealValue {
  match possibly_label {
    Value::Label(x) => resolve_label(&x),
    Value::Real(x) => x,
  }
}

pub fn invoke(
  side_effector: &mut dyn side_effects::SideEffector,
  stack: &mut Vec<Value>,
) {
  match stack.pop().map(realize_value) {
    Some(RealValue::Fun(x)) => match x {
      Function::SideEffect(y) => {
        side_effector.invoke(y, stack)
      }
    }
    x => panic!("Bad input to invoke: {:?}", x),
  }
}
use std::error::Error;
use std::collections::HashMap;

// We create side-effects through a trait implementation
// (This allows mocking all side effects in one for testing)
mod parse;
use parse::*;
pub mod invoke;
use invoke::{
  invoke,
  resolve_label,
  realize_value,
  side_effects::{
    SideEffector,
    SideEffectFunction,
  },
};

mod types;
pub use types::{
  Value,
  RealValue,
  ProgramValue,
  Function,
  Template,
  TemplateValue,
};

pub fn interpret<'a>(
  mut program: Vec<ProgramValue>,
  side_effector: &mut dyn SideEffector,
  global_scope: HashMap<String, RealValue>,
) -> Result<Vec<Value>, Box<dyn Error>> {
  // Repeatedly pop the next program instruction off the program stack and
  // interpret it
  let mut data_stack: Vec<Value> = Vec::new();
  use ProgramValue as PV;
  for operation in program { match operation {
    PV::Real(v) => { data_stack.push(Value::Real(v)); },
    PV::Label(l) => { data_stack.push(Value::Label(l)); },
    PV::Template{consumed_stack_entries, source} => {
      if consumed_stack_entries > data_stack.len() {
        panic!("Template consumes more stack entries than there are.");
      }
      let consumed_stack = data_stack.split_off(
        data_stack.len() - consumed_stack_entries
      );
      data_stack.push(render_template(
        consumed_stack,
        &global_scope,
        source,
      ).into());
    },
    PV::Invoke => { invoke(side_effector, &mut data_stack); },
  } }
  Ok(data_stack)
}

pub fn interpret_str(
  script: &str,
  side_effector: &mut dyn SideEffector,
) -> Result<Vec<Value>, Box<dyn Error>> {
  let mut scope = HashMap::new();
  interpret(
    parse_str(script),
    side_effector,
    scope,
  )
}

