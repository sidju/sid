use std::collections::HashMap;

use super::{
  ProgramValue,
  DataValue,
  TemplateValue,
  GlobalState,
  InterpretBuiltIn,
  render_template,
  call_c_function,
  call_cfuncsig,
};

pub struct ExeState<'a> {
  pub program_stack: Vec<ProgramValue>,
  pub data_stack: Vec<TemplateValue>,
  pub local_scope: HashMap<String, DataValue>,
  pub global_state: GlobalState<'a>,
}

pub fn invoke<'a, 'b>(
  data_stack: &mut Vec<TemplateValue>,
  program_stack: &mut Vec<ProgramValue>,
  local_scope: &mut HashMap<String, DataValue>,
  global_state: &mut GlobalState<'a>,
  builtins: &HashMap<&'b str, &'b dyn InterpretBuiltIn>,
) {
  // Resolve labels before invoking
  let value = match data_stack.pop() {
    Some(TemplateValue::Literal(ProgramValue::Data(DataValue::Label(l)))) => {
      if let Some(v) = local_scope.get(&l) { v.clone() }
      else if let Some(v) = global_state.scope.get(&l) { v.clone() }
      else if builtins.contains_key(&l.as_str()) { DataValue::BuiltIn(l) }
      else { panic!("Undefined label dereference: {}", l); }
    },
    Some(TemplateValue::Literal(ProgramValue::Data(v))) => v,
    Some(other) => panic!("Invoked on non-data stack entry: {:?}", other),
    None => panic!("Invoked on empty data_stack!"),
  };
  match value {
    // Invoking a substack puts it on your program_stack and resumes execution
    DataValue::Substack { body: mut s, .. } => {
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
      for result in builtin.execute(data_stack, global_state, program_stack)
        .unwrap_or_else(|e| panic!("BuiltIn '{}' returned error: {}", function, e))
      {
        data_stack.push(TemplateValue::from(result));
      }
    },
    // Invoking a linked CFuncSig: look up the symbol by name at call time.
    DataValue::CFuncSig(sig) => {
      let param_count = sig.params.len();
      // Variadic functions always take a List (fixed + variadic args together).
      let arg = if sig.variadic || param_count > 1 {
        match data_stack.pop() {
          Some(TemplateValue::Literal(ProgramValue::Data(DataValue::List(items)))) =>
            Some(DataValue::List(items)),
          Some(other) => panic!(
            "CFuncSig '{}': expected List, got {:?}",
            sig.name, other
          ),
          None => panic!("CFuncSig '{}': expected argument but stack was empty", sig.name),
        }
      } else if param_count == 1 {
        match data_stack.pop() {
          Some(TemplateValue::Literal(ProgramValue::Data(v))) => Some(v),
          Some(other) => panic!("CFuncSig '{}': argument is not a concrete value: {:?}", sig.name, other),
          None => panic!("CFuncSig '{}': expected argument but stack was empty", sig.name),
        }
      } else {
        None
      };
      if let Some(result) = call_cfuncsig(&sig, arg, &global_state.libraries)
        .unwrap_or_else(|e| panic!("CFuncSig '{}' call error: {}", sig.name, e))
      {
        data_stack.push(TemplateValue::from(result));
      }
    },
    // Invoking a dynamically-loaded C function via libffi.
    DataValue::CFunction(f) => {
      let param_count = f.sig.params.len();
      let arg = if param_count == 0 {
        None
      } else if param_count == 1 {
        match data_stack.pop() {
          Some(TemplateValue::Literal(ProgramValue::Data(v))) => Some(v),
          Some(other) => panic!("CFunction '{}': argument is not a concrete value: {:?}", f.name, other),
          None => panic!("CFunction '{}': expected argument but stack was empty", f.name),
        }
      } else {
        // Multiple params: expect a List.
        match data_stack.pop() {
          Some(TemplateValue::Literal(ProgramValue::Data(DataValue::List(items)))) =>
            Some(DataValue::List(items)),
          Some(other) => panic!(
            "CFunction '{}': expected List for {} params, got {:?}",
            f.name, param_count, other
          ),
          None => panic!("CFunction '{}': expected argument but stack was empty", f.name),
        }
      };
      if let Some(result) = call_c_function(&f, arg)
        .unwrap_or_else(|e| panic!("CFunction '{}' returned error: {}", f.name, e))
      {
        data_stack.push(TemplateValue::from(result));
      }
    },
    _ => panic!("Invalid object invoked.")
  }
}

pub fn interpret<'a, 'b>(
  program: Vec<ProgramValue>,
  data_stack: Vec<TemplateValue>,
  global_state: GlobalState<'a>,
  builtins: &HashMap<&'b str, &'b dyn InterpretBuiltIn>,
) {
  let local_scope = HashMap::new();
  let mut exe_state = ExeState {
    program_stack: program,
    data_stack,
    local_scope,
    global_state,
  };
  while !exe_state.program_stack.is_empty() {
    interpret_one(&mut exe_state, builtins)
  }
}

pub fn interpret_one<'a, 'b>(
  exe_state: &mut ExeState<'a>,
  builtins: &HashMap<&'b str, &'b dyn InterpretBuiltIn>,
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
        &exe_state.global_state.scope,
      );
      exe_state.data_stack.extend(rendered.into_iter().map(TemplateValue::from));
    },
    PV::Invoke | PV::ComptimeInvoke => { invoke(
      &mut exe_state.data_stack,
      &mut exe_state.program_stack,
      &mut exe_state.local_scope,
      &mut exe_state.global_state,
      builtins,
    ); },
    PV::StackSizeAssert { expected_len, message } => {
      if exe_state.data_stack.len() != expected_len {
        panic!(
          "{} (expected stack size {}, got {})",
          message, expected_len, exe_state.data_stack.len()
        );
      }
    },
    PV::CondLoop { cond, body, expected_len } => {
      if exe_state.data_stack.len() != expected_len + 1 {
        panic!(
          "loop condition must leave exactly one Bool on top (expected stack size {}, got {})",
          expected_len + 1, exe_state.data_stack.len()
        );
      }
      let bool_val = match exe_state.data_stack.pop() {
        Some(TemplateValue::Literal(ProgramValue::Data(DataValue::Bool(b)))) => b,
        Some(other) => panic!(
          "loop condition must leave a Bool on top of the stack, got {:?}", other
        ),
        None => unreachable!(),
      };
      if bool_val {
        let mut body_rev: Vec<ProgramValue> = body.iter().rev().cloned().collect();
        let mut cond_rev: Vec<ProgramValue> = cond.iter().rev().cloned().collect();
        // Push in reverse execution order: CondLoop (last) → cond → StackSizeAssert → body (first).
        exe_state.program_stack.push(PV::CondLoop { cond, body, expected_len });
        exe_state.program_stack.append(&mut cond_rev);
        exe_state.program_stack.push(PV::StackSizeAssert {
          expected_len,
          message: "loop body must leave the stack unchanged",
        });
        exe_state.program_stack.append(&mut body_rev);
      }
    },
  }
}
