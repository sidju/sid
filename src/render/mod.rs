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
      let v = get_from_scope(&l, Some(parent_scope), Some(global_scope), Some(builtins))
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

/// Evaluate a resolved `ProgramValue` sequence on a fresh data stack,
/// reusing the full interpreter dispatch via `interpret_one`. Used to evaluate
/// multi-token Map key/value expressions inline at render time.
fn eval_data_seq(
  pvs: Vec<ProgramValue>,
  parent_scope: &HashMap<String, DataValue>,
  global_state: &mut GlobalState,
  builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
) -> Vec<DataValue> {
  use crate::invoke::interpret_one;
  let mut data_stack: Vec<TemplateValue> = Vec::new();
  let mut program_stack: Vec<ProgramValue> = pvs.into_iter().rev().collect();
  let mut local_scope = parent_scope.clone();
  let mut scope_stack: Vec<HashMap<String, DataValue>> = Vec::new();
  while !program_stack.is_empty() {
    interpret_one(&mut data_stack, &mut program_stack, &mut local_scope, &mut scope_stack, global_state, builtins);
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
      DataValue::List(eval_data_seq(pvs, parent_scope, global_state, builtins))
    },
    TD::Set(source) => {
      let pvs = resolve_to_program_values(source, &mut consumed_stack, parent_scope, global_state.scope, builtins);
      DataValue::Set(eval_data_seq(pvs, parent_scope, global_state, builtins))
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
