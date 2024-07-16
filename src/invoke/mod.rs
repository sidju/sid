use std::collections::HashMap;

use super::{
  ProgramValue,
  RealValue,
  DataValue,
  BuiltInFunction,
  render_template,
};


// Invoking behaves very differently depending on what is invoked
pub fn invoke<'a>(
  data_stack: &mut Vec<DataValue>,
  program_stack: &mut Vec<ProgramValue>,
  local_scope: &mut HashMap<String, RealValue>,
  global_scope: &mut HashMap<String, RealValue>,
  built_in_functions: &HashMap<&'a str, &'a dyn BuiltInFunction>,
) {
  let value = match data_stack.pop() {
    Some(DataValue::Real(v)) => v,
    Some(DataValue::Label(l)) => {
      if let Some(v) = local_scope.get(&l) { v.clone() }
      else if let Some(v) = global_scope.get(&l) { v.clone() }
      else { panic!("Undefined label dereference: {}", l); }
    },
    None => panic!("Invoked on empty data_stack!"),
  };
  match value {
    // Invoking a substack puts it on your program_stack and resumes execution
    // (This means it executes in current context / has access to local scope)
    RealValue::Substack(mut s) => {
      program_stack.append(&mut s);
    },

    // Invoking a function interprets the function on your data_stack and
    // global_scope, but without access to your local scope or program_stack
    // TODO

    // Invoking a built-in function might do anything, but usually acts like a
    // normal function
    RealValue::BuiltInFunction(function) => {
      built_in_functions[&function[..]].execute(
        data_stack,
        program_stack,
        local_scope,
        global_scope,
      );
    },
    _ => panic!("Invalid object invoked.")
  }
}

// Repeatedly pop and interpret each value from the program stack
pub fn interpret<'a>(
  mut program: Vec<ProgramValue>,
  data_stack: &mut Vec<DataValue>,
  mut global_scope: HashMap<String, RealValue>,
  built_in_functions: &HashMap<&'a str, &'a dyn BuiltInFunction>,
) {
  let mut local_scope = HashMap::new();
  use ProgramValue as PV;
  while let Some(operation) = program.pop() { 
    match operation {
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
        &mut local_scope,
        &mut global_scope,
        built_in_functions,
      ); },
    } 
  }
}
