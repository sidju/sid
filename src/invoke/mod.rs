use std::collections::HashMap;

use super::{
  ProgramValue,
  DataValue,
  TemplateValue,
  InterpretBuiltIn,
  render_template,
};

pub struct ExeState {
  pub program_stack: Vec<ProgramValue>,
  pub data_stack: Vec<TemplateValue>,
  pub local_scope: HashMap<String, DataValue>,
  pub global_scope: HashMap<String, DataValue>,
}

// Invoking behaves very differently depending on what is invoked
pub fn invoke<'a>(
  data_stack: &mut Vec<TemplateValue>,
  program_stack: &mut Vec<ProgramValue>,
  local_scope: &mut HashMap<String, DataValue>,
  global_scope: &mut HashMap<String, DataValue>,
  builtins: &HashMap<&'a str, &'a dyn InterpretBuiltIn>,
) {
  // Resolve labels before invoking
  let value = match data_stack.pop() {
    Some(TemplateValue::Literal(ProgramValue::Data(DataValue::Label(l)))) => {
      if let Some(v) = local_scope.get(&l) { v.clone() }
      else if let Some(v) = global_scope.get(&l) { v.clone() }
      else if builtins.contains_key(&l.as_str()) { DataValue::BuiltIn(l) }
      else { panic!("Undefined label dereference: {}", l); }
    },
    Some(TemplateValue::Literal(ProgramValue::Data(v))) => v,
    Some(other) => panic!("Invoked on non-data stack entry: {:?}", other),
    None => panic!("Invoked on empty data_stack!"),
  };
  match value {
    // Invoking a substack puts it on your program_stack and resumes execution
    DataValue::Substack(mut s) => {
      s.reverse();
      program_stack.append(&mut s);
    },

    // Invoking a function interprets the function on your data_stack and
    // global_scope, but without access to your local scope or program_stack
    // TODO

    // Invoking a built-in via the InterpretBuiltIn trait:
    // arg_count/return_count determine stack interaction.
    DataValue::BuiltIn(function) => {
      let builtin = builtins[&function[..]];
      let arg = if builtin.arg_count() == 1 {
        match data_stack.pop() {
          Some(TemplateValue::Literal(ProgramValue::Data(v))) => Some(v),
          Some(other) => panic!("BuiltIn '{}': argument is not a concrete value: {:?}", function, other),
          None => panic!("BuiltIn '{}': expected argument but stack was empty", function),
        }
      } else {
        None
      };
      if let Some(result) = builtin.execute(arg, global_scope)
        .unwrap_or_else(|e| panic!("BuiltIn '{}' returned error: {}", function, e))
      {
        data_stack.push(TemplateValue::from(result));
      }
    },
    _ => panic!("Invalid object invoked.")
  }
}

pub fn interpret<'a>(
  program: Vec<ProgramValue>,
  data_stack: Vec<TemplateValue>,
  global_scope: HashMap<String, DataValue>,
  builtins: &HashMap<&'a str, &'a dyn InterpretBuiltIn>,
) {
  let local_scope = HashMap::new();
  let mut exe_state = ExeState {
    program_stack: program,
    data_stack,
    local_scope,
    global_scope,
  };
  while !exe_state.program_stack.is_empty() {
    interpret_one(&mut exe_state, builtins)
  }
}

pub fn interpret_one<'a>(
  exe_state: &mut ExeState,
  builtins: &HashMap<&'a str, &'a dyn InterpretBuiltIn>,
) {
  use ProgramValue as PV;
  let operation = exe_state.program_stack.pop().unwrap();
  match operation {
    PV::Data(v) => { exe_state.data_stack.push(TemplateValue::Literal(PV::Data(v))); },
    PV::Template(t) => {
      let rendered = render_template(
        t,
        &mut exe_state.data_stack,
        &exe_state.local_scope,
        &exe_state.global_scope,
      );
      exe_state.data_stack.extend(rendered.into_iter().map(TemplateValue::from));
    },
    PV::Invoke | PV::ComptimeInvoke => { invoke(
      &mut exe_state.data_stack,
      &mut exe_state.program_stack,
      &mut exe_state.local_scope,
      &mut exe_state.global_scope,
      builtins,
    ); },
  }
}
