use std::collections::HashMap;

use super::{
  ProgramValue,
  RealValue,
  DataValue,
  render_template,
};


// Invoking behaves very differently depending on what is invoked
pub fn invoke(
  data_stack: &mut Vec<DataValue>,
  program_stack: &mut Vec<ProgramValue>,
  parent_scope: &HashMap<String, RealValue>,
  global_scope: &HashMap<String, RealValue>,
) {
  match match data_stack.pop() {
    Some(DataValue::Real(v)) => v,
    Some(DataValue::Label(l)) => {
      if let Some(v) = parent_scope.get(&l) { v.clone() }
      else if let Some(v) = global_scope.get(&l) { v.clone() }
      else { panic!("Undefined label dereference: {}", l); }
    },
    None => panic!("Invoked on empty data_stack!"),
  } {
    // Invoking a substack puts it on your program_stack and resumes execution
    RealValue::Substack(mut s) => {
      program_stack.append(&mut s);
    },

    // TODO
    // Invoking a function interprets the function on your data_stack and
    // global_scope, but with its own local_scope.

    // Invoking a built-in function might do anything
    _ => panic!("Invalid object invoked.")
  }
}

// Repeatedly pop and interpret each value from the program stack
pub fn interpret(
  mut program: Vec<ProgramValue>,
  data_stack: &mut Vec<DataValue>,
  global_scope: HashMap<String, RealValue>,
) {
  let local_scope = HashMap::new();
  use ProgramValue as PV;
  while let Some(operation) = program.pop() { match operation {
    PV::Real(v) => { data_stack.push(DataValue::Real(v)); },
    PV::Label(l) => { data_stack.push(DataValue::Label(l)); },
    PV::Template(t) => {
      let mut rendered = render_template(
        t,
        data_stack,
        &local_scope,
        &global_scope,
      );
      data_stack.append(&mut rendered);
    },
    PV::Invoke => { invoke(
      data_stack,
      &mut program,
      &global_scope,
      &local_scope,
    ); },
  } }
}
