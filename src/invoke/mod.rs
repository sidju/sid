use std::collections::HashMap;

use super::{
  ProgramValue,
  DataValue,
  TemplateValue,
  GlobalState,
  InterpretBuiltIn,
  SidType,
  render_template,
  call_c_function,
  call_cfuncsig,
  get_from_scope,
};

/// Check the top of `data_stack` against a type slice.
///
/// `types[0]` = top of stack, `types[N-1]` = deepest checked item.
/// Panics with a detailed message on the first mismatch, or if the stack is
/// too shallow. `label` ("args"/"ret") and `context` (callable description)
/// are included in any panic message.
fn check_type_contract(
  data_stack: &[TemplateValue],
  types: &[SidType],
  label: &str,
  context: &str,
) {
  if data_stack.len() < types.len() {
    panic!(
      "{} {} check failed: expected {} items on stack, only {} available",
      context, label, types.len(), data_stack.len()
    );
  }
  for (i, (tv, expected)) in data_stack.iter().rev().zip(types.iter()).enumerate() {
    let actual = match tv {
      TemplateValue::Literal(ProgramValue::Data(v)) => v,
      other => panic!(
        "{} {} check failed: position {} (0=top): expected {:?}, got non-concrete value {:?}",
        context, label, i, expected, other
      ),
    };
    if !expected.matches(actual) {
      panic!(
        "{} {} check failed: position {} (0=top): expected {:?}, got {:?}",
        context, label, i, expected, actual
      );
    }
  }
}


pub struct ExeState<'a> {
  pub program_stack: Vec<ProgramValue>,
  pub data_stack: Vec<TemplateValue>,
  pub local_scope: HashMap<String, DataValue>,
  /// Scope stack used by `PushScope`/`PopScope` sentinels to isolate substack
  /// local bindings.  Each `PushScope` pushes the current `local_scope` here
  /// and installs a fresh empty one; `PopScope` restores it.
  pub scope_stack: Vec<HashMap<String, DataValue>>,
  pub global_state: GlobalState<'a>,
}

pub fn invoke<'a, 'b>(
  data_stack: &mut Vec<TemplateValue>,
  program_stack: &mut Vec<ProgramValue>,
  local_scope: &mut HashMap<String, DataValue>,
  global_state: &mut GlobalState<'a>,
  builtins: &HashMap<&'b str, &'b dyn InterpretBuiltIn>,
) {
  // Resolve labels via scope, falling back to built-ins at lowest priority.
  let value = match data_stack.pop() {
    Some(TemplateValue::Literal(ProgramValue::Data(DataValue::Label(l)))) =>
      get_from_scope(&l, Some(local_scope), global_state.scope, builtins)
        .expect("label resolution failed"),
    Some(TemplateValue::Literal(ProgramValue::Data(v))) => v,
    Some(other) => panic!("Invoked on non-data stack entry: {:?}", other),
    None => panic!("Invoked on empty data_stack!"),
  };
  match value {
    // Invoking a substack: check arg types (if declared), schedule ret check
    // (if declared), then push the body onto the program stack.
    // Both args and ret are stored top-first (index 0 = top of stack).
    //
    // Every substack gets a fresh local scope (PushScope / PopScope sentinels).
    // When `args` are declared a `StackBlock` is inserted below the top N items
    // on the data stack so the body cannot accidentally read the caller's stack.
    // The `TypeCheck` sentinel (with `block_placed: true`) fires after the body
    // and PopScope to remove the block and optionally verify return types.
    DataValue::Substack { body: mut s, args, ret } => {
      s.reverse();
      let block_placed = args.is_some();
      let names: Vec<String> = args.as_ref()
        .map(|a| a.iter().map(|(n, _)| n.clone()).collect())
        .unwrap_or_default();
      if let Some(ref arg_fields) = args {
        let arg_types: Vec<SidType> = arg_fields.iter().map(|(_, t)| t.clone()).collect();
        check_type_contract(data_stack, &arg_types, "args", "substack");
        // Insert StackBlock below the args; args will be consumed by PushScope.
        let n = arg_fields.len();
        let insert_pos = data_stack.len() - n;
        data_stack.insert(insert_pos, TemplateValue::from(DataValue::StackBlock));
      }
      // Schedule cleanup / ret check (fires last, after PopScope).
      match (&args, &ret) {
        (None, None) => {},
        _ => program_stack.push(ProgramValue::TypeCheck {
          types: ret,
          context: "substack ret".to_owned(),
          block_placed,
        }),
      }
      program_stack.push(ProgramValue::PopScope);
      program_stack.append(&mut s);
      program_stack.push(ProgramValue::PushScope { names });
    },

    // Invoking a built-in via the InterpretBuiltIn trait:
    // arg_count/return_count determine stack interaction.
    DataValue::BuiltIn(function) => {
      let builtin = builtins[&function[..]];
      for result in builtin.execute(data_stack, global_state, program_stack, local_scope, builtins)
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
    scope_stack: Vec::new(),
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
        builtins,
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
    PV::TypeCheck { types, context, block_placed } => {
      if block_placed {
        // Find the StackBlock inserted at invocation time.
        let block_pos = exe_state.data_stack.iter().rposition(|tv| {
          matches!(tv, TemplateValue::Literal(ProgramValue::Data(DataValue::StackBlock)))
        }).unwrap_or_else(|| panic!("TypeCheck ({}): no StackBlock found on data stack", context));
        if let Some(ret_types) = types {
          let results = &exe_state.data_stack[block_pos + 1..];
          if results.len() != ret_types.len() {
            panic!(
              "TypeCheck ({}): expected {} return value(s) above StackBlock, got {}",
              context, ret_types.len(), results.len()
            );
          }
          check_type_contract(results, &ret_types, "ret", &context);
        }
        exe_state.data_stack.remove(block_pos);
      } else if let Some(ret_types) = types {
        check_type_contract(&exe_state.data_stack, &ret_types, "ret", &context);
      }
    },
    PV::PushScope { names } => {
      let old_scope = std::mem::replace(&mut exe_state.local_scope, HashMap::new());
      exe_state.scope_stack.push(old_scope);
      // Consume args from the top of the data stack (top-first order matches names[0]).
      for name in names.into_iter() {
        let value = match exe_state.data_stack.pop() {
          Some(TemplateValue::Literal(ProgramValue::Data(v))) => v,
          Some(other) => panic!("PushScope: arg '{}' is not a concrete value: {:?}", name, other),
          None => panic!("PushScope: expected arg '{}' but data stack was empty", name),
        };
        exe_state.local_scope.insert(name, value);
      }
    },
    PV::PopScope => {
      exe_state.local_scope = exe_state.scope_stack.pop()
        .expect("PopScope with no matching PushScope");
    },
  }
}
