use std::collections::HashMap;

use crate::{
  Template,
  TemplateData,
  TemplateValue,
  DataValue,
  ProgramValue,
  GlobalState,
  InterpretBuiltIn,
  get_from_scope,
};

/// Resolve a sequence of `TemplateValue`s into `ProgramValue`s, consuming
/// parent stack slots and looking up parent labels as needed.
fn resolve_to_program_values(
  source: Vec<TemplateValue>,
  consumed_stack: &mut Vec<Option<DataValue>>,
  parent_scope: &HashMap<String, DataValue>,
  global_scope: &HashMap<String, DataValue>,
  builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
) -> Vec<ProgramValue> {
  let mut rendered: Vec<ProgramValue> = Vec::new();
  use TemplateValue as TV;
  for entry in source { match entry {
    TV::Literal(v) => { rendered.push(v); },
    TV::ParentLabel(l) => {
      let v = get_from_scope(&l, Some(parent_scope), global_scope, builtins)
        .expect("label resolution failed");
      rendered.push(v.into())
    },
    TV::ParentStackMove(i) => {
      if i == 0 { panic!("Parent stack index 0 is the template!"); }
      let value = consumed_stack[i - 1]
        .take()
        .expect("Stack value taken twice in template");
      rendered.push(value.into());
    },
  }}
  rendered
}

/// Convert a resolved `ProgramValue` sequence into `DataValue`s for use inside
/// data container literals (List, Set).
///
/// Nested templates are rendered recursively — for substacks/scripts the body
/// stays as `Vec<ProgramValue>` (unexecuted).
/// Invoke tokens and control-flow sentinels panic.
fn program_values_to_data_values(
  program_values: Vec<ProgramValue>,
  parent_scope: &HashMap<String, DataValue>,
  global_state: &mut GlobalState,
  builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
) -> Vec<DataValue> {
  program_values.into_iter().flat_map(|pv| match pv {
    ProgramValue::Data(v) => vec![v],
    ProgramValue::Template(t) =>
      render_template(t, &mut Vec::new(), parent_scope, global_state, builtins),
    ProgramValue::Invoke | ProgramValue::ComptimeInvoke =>
      panic!("Invoke token found where a data value was expected"),
    ProgramValue::CondLoop { .. } =>
      panic!("CondLoop sentinel found where a data value was expected"),
    ProgramValue::StackSizeAssert { .. } =>
      panic!("StackSizeAssert sentinel found where a data value was expected"),
    ProgramValue::TypeCheck { .. } =>
      panic!("TypeCheck sentinel found where a data value was expected"),
    ProgramValue::PushScope { .. } =>
      panic!("PushScope sentinel found where a data value was expected"),
    ProgramValue::PopScope =>
      panic!("PopScope sentinel found where a data value was expected"),
    ProgramValue::CondLoopStart { .. } =>
      panic!("CondLoopStart sentinel found where a data value was expected"),
  }).collect()
}

/// Run a mini interpreter loop over `pvs` on a fresh data stack.
///
/// Used to evaluate multi-token Map key/value expressions inline at render
/// time.  Supports `invoke`, nested `render_template`, scope sentinels, and
/// type-check sentinels, but panics on control-flow sentinels (CondLoop, etc.)
/// that have no meaning inside a data template expression.
fn eval_data_seq(
  pvs: Vec<ProgramValue>,
  parent_scope: &HashMap<String, DataValue>,
  global_state: &mut GlobalState,
  builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
) -> Vec<DataValue> {
  use crate::invoke::invoke;
  use crate::invoke::check_type_contract;
  let mut data_stack: Vec<TemplateValue> = Vec::new();
  let mut program_stack: Vec<ProgramValue> = pvs.into_iter().rev().collect();
  let mut local_scope = parent_scope.clone();
  let mut scope_stack: Vec<HashMap<String, DataValue>> = Vec::new();
  while let Some(pv) = program_stack.pop() {
    match pv {
      ProgramValue::Data(v) => {
        data_stack.push(TemplateValue::Literal(ProgramValue::Data(v)));
      }
      ProgramValue::Template(t) => {
        let rendered = render_template(t, &mut data_stack, &local_scope, global_state, builtins);
        data_stack.extend(rendered.into_iter().map(TemplateValue::from));
      }
      ProgramValue::Invoke | ProgramValue::ComptimeInvoke => {
        invoke(&mut data_stack, &mut program_stack, &mut local_scope, global_state, builtins);
      }
      ProgramValue::PushScope { names } => {
        let old = std::mem::replace(&mut local_scope, HashMap::new());
        scope_stack.push(old);
        for name in names {
          let v = match data_stack.pop() {
            Some(TemplateValue::Literal(ProgramValue::Data(v))) => v,
            Some(other) => panic!("eval_data_seq PushScope: arg '{}' not concrete: {:?}", name, other),
            None => panic!("eval_data_seq PushScope: expected arg '{}' but stack empty", name),
          };
          local_scope.insert(name, v);
        }
      }
      ProgramValue::PopScope => {
        local_scope = scope_stack.pop().expect("eval_data_seq PopScope: no matching PushScope");
      }
      ProgramValue::TypeCheck { types, context, block_placed } => {
        if block_placed {
          let block_pos = data_stack.iter().rposition(|tv| {
            matches!(tv, TemplateValue::Literal(ProgramValue::Data(DataValue::StackBlock)))
          }).unwrap_or_else(|| panic!("eval_data_seq TypeCheck ({}): no StackBlock on stack", context));
          if let Some(ret_types) = types {
            let results_len = data_stack.len() - (block_pos + 1);
            if results_len != ret_types.len() {
              panic!(
                "eval_data_seq TypeCheck ({}): expected {} return value(s) above StackBlock, got {}",
                context, ret_types.len(), results_len
              );
            }
            let results_mut = &mut data_stack[block_pos + 1..];
            check_type_contract(results_mut, &ret_types, "ret", &context, &local_scope, global_state.scope, builtins);
          }
          data_stack.remove(block_pos);
        } else if let Some(ret_types) = types {
          check_type_contract(&mut data_stack, &ret_types, "ret", &context, &local_scope, global_state.scope, builtins);
        }
      }
      ProgramValue::StackSizeAssert { expected_len, message } => {
        if data_stack.len() != expected_len {
          panic!("{} (expected stack size {}, got {})", message, expected_len, data_stack.len());
        }
      }
      other => panic!("eval_data_seq: unexpected ProgramValue in data template context: {:?}", other),
    }
  }
  data_stack.into_iter().map(|tv| match tv {
    TemplateValue::Literal(ProgramValue::Data(v)) => v,
    other => panic!("eval_data_seq: non-data value left on mini-stack: {:?}", other),
  }).collect()
}

pub fn render_template(
  template: Template,
  parent_stack: &mut Vec<TemplateValue>,
  parent_scope: &HashMap<String, DataValue>,
  global_state: &mut GlobalState,
  builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
) -> Vec<DataValue> {
  if template.consumes_stack_entries > parent_stack.len() {
    panic!("Template consumes more stack entries than there are.");
  }
  let mut consumed_stack: Vec<Option<DataValue>> = parent_stack
    .drain(parent_stack.len() - template.consumes_stack_entries ..)
    .map(|tv| match tv {
      TemplateValue::Literal(ProgramValue::Data(v)) => v,
      other => panic!("render_template: parent stack entry is not a concrete DataValue: {:?}", other),
    })
    .map(|v| Some(v))
    .collect();

  use TemplateData as TD;
  let rendered_template: DataValue = match template.data {
    TD::Substack(source) => DataValue::Substack {
      body: resolve_to_program_values(source, &mut consumed_stack, parent_scope, global_state.scope, builtins),
      args: None,
      ret: None,
    },
    TD::Script(source) => DataValue::Script {
      body: resolve_to_program_values(source, &mut consumed_stack, parent_scope, global_state.scope, builtins),
      args: None,
      ret: None,
    },
    TD::List(source) => {
      let pvs = resolve_to_program_values(source, &mut consumed_stack, parent_scope, global_state.scope, builtins);
      DataValue::List(program_values_to_data_values(pvs, parent_scope, global_state, builtins))
    },
    TD::Set(source) => {
      let pvs = resolve_to_program_values(source, &mut consumed_stack, parent_scope, global_state.scope, builtins);
      DataValue::Set(program_values_to_data_values(pvs, parent_scope, global_state, builtins))
    },
    TD::Map(pairs) => {
      let mut entries: Vec<(DataValue, DataValue)> = Vec::new();
      for (key_tvs, val_tvs) in pairs {
        let key_pvs = resolve_to_program_values(key_tvs, &mut consumed_stack, parent_scope, global_state.scope, builtins);
        let val_pvs = resolve_to_program_values(val_tvs, &mut consumed_stack, parent_scope, global_state.scope, builtins);
        let mut key_vals = eval_data_seq(key_pvs, parent_scope, global_state, builtins);
        let mut val_vals = eval_data_seq(val_pvs, parent_scope, global_state, builtins);
        if key_vals.len() != 1 {
          panic!("map key expression must produce exactly 1 value, got {}", key_vals.len());
        }
        if val_vals.len() != 1 {
          panic!("map value expression must produce exactly 1 value, got {}", val_vals.len());
        }
        entries.push((key_vals.remove(0), val_vals.remove(0)));
      }
      DataValue::Map(entries)
    },
  };

  let mut rendered_stack: Vec<DataValue> = consumed_stack.drain(..)
    .filter_map(|x| x)
    .collect();
  rendered_stack.push(rendered_template);
  rendered_stack
}
