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
      else if built_in_functions.contains_key(&l.as_str()) { RealValue::BuiltInFunction(l) }
      else { panic!("Undefined label dereference: {}", l); }
    },
    None => panic!("Invoked on empty data_stack!"),
  };
  match value {
    // Invoking a substack puts it on your program_stack and resumes execution
    // (This means it executes in current context / has access to local scope)
    RealValue::Substack(mut s) => {
      s.reverse();
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
  program: Vec<ProgramValue>,
  data_stack: Vec<DataValue>,
  global_scope: HashMap<String, RealValue>,
  built_in_functions: &HashMap<&'a str, &'a dyn BuiltInFunction>,
) {
  let local_scope = HashMap::new();
  let mut exe_state = ExeState {
    program_stack: program,
    data_stack,
    local_scope,
    global_scope,
  };
  while !exe_state.program_stack.is_empty() {   
    interpret_one(
      &mut exe_state,
      built_in_functions)
  }
}


pub struct ExeState {
  pub program_stack: Vec<ProgramValue>,
  pub data_stack: Vec<DataValue>,
  pub local_scope: HashMap<String, RealValue>,
  pub global_scope: HashMap<String, RealValue>,
}

pub fn interpret_one<'a>(
  exe_state: &mut ExeState,
  built_in_functions: &HashMap<&'a str, &'a dyn BuiltInFunction>,
) {
  use ProgramValue as PV;
  let operation = exe_state.program_stack.pop().unwrap();
  match operation {
    PV::Real(v) => { exe_state.data_stack.push(DataValue::Real(v)); },
    PV::Label(l) => { exe_state.data_stack.push(DataValue::Label(l)); },
    PV::Template(t) => {
      let mut rendered = render_template(
        t,
        &mut exe_state.data_stack,
        &exe_state.local_scope,
        &exe_state.global_scope,
      );
      exe_state.data_stack.append(&mut rendered);
    },
    PV::Invoke => { invoke(
      &mut exe_state.data_stack,
      &mut exe_state.program_stack,
      &mut exe_state.local_scope,
      &mut exe_state.global_scope,
      built_in_functions,
    ); },
  }
}
