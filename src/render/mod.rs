use std::collections::HashMap;

use crate::{
  Template,
  TemplateData,
  TemplateValue,
  DataValue,
  ProgramValue,
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
/// data container literals (List, Set, Map).
///
/// Nested templates (e.g. a substack `(42)` inside a list) are rendered
/// recursively — for substacks/scripts the body stays as `Vec<ProgramValue>`
/// (unexecuted); the wrapper is simply created here.
/// Invoke tokens and control-flow sentinels panic.
fn program_values_to_data_values(
  program_values: Vec<ProgramValue>,
  parent_scope: &HashMap<String, DataValue>,
  global_scope: &HashMap<String, DataValue>,
  builtins: &HashMap<&str, &dyn InterpretBuiltIn>,
) -> Vec<DataValue> {
  program_values.into_iter().flat_map(|pv| match pv {
    ProgramValue::Data(v) => vec![v],
    ProgramValue::Template(t) =>
      render_template(t, &mut Vec::new(), parent_scope, global_scope, builtins),
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
  }).collect()
}

pub fn render_template(
  template: Template,
  parent_stack: &mut Vec<TemplateValue>,
  parent_scope: &HashMap<String, DataValue>,
  global_scope: &HashMap<String, DataValue>,
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
      body: resolve_to_program_values(source, &mut consumed_stack, parent_scope, global_scope, builtins),
      args: None,
      ret: None,
    },
    TD::Script(source) => DataValue::Script {
      body: resolve_to_program_values(source, &mut consumed_stack, parent_scope, global_scope, builtins),
      args: None,
      ret: None,
    },
    TD::List(source) => {
      let pvs = resolve_to_program_values(source, &mut consumed_stack, parent_scope, global_scope, builtins);
      DataValue::List(program_values_to_data_values(pvs, parent_scope, global_scope, builtins))
    },
    TD::Set(source) => {
      let pvs = resolve_to_program_values(source, &mut consumed_stack, parent_scope, global_scope, builtins);
      DataValue::Set(program_values_to_data_values(pvs, parent_scope, global_scope, builtins))
    },
    TD::Map(pairs) => {
      let mut entries: Vec<(DataValue, DataValue)> = Vec::new();
      for (key_tv, val_tv) in pairs {
        let key_pvs = resolve_to_program_values(vec![key_tv], &mut consumed_stack, parent_scope, global_scope, builtins);
        let val_pvs = resolve_to_program_values(vec![val_tv], &mut consumed_stack, parent_scope, global_scope, builtins);
        let key = program_values_to_data_values(key_pvs, parent_scope, global_scope, builtins).into_iter().next().expect("empty map key");
        let val = program_values_to_data_values(val_pvs, parent_scope, global_scope, builtins).into_iter().next().expect("empty map value");
        entries.push((key, val));
      }
      // If every key is a Label the literal reads as a struct (named fields);
      // otherwise it is a plain map.  Both are represented as DataValue::Map —
      // the key type alone distinguishes them.
      DataValue::Map(entries)
    },
  };

  let mut rendered_stack: Vec<DataValue> = consumed_stack.drain(..)
    .filter_map(|x| x)
    .collect();
  rendered_stack.push(rendered_template);
  rendered_stack
}
