use std::collections::HashMap;

use crate::{
  Template,
  TemplateData,
  TemplateValue,
  DataValue,
  ProgramValue,
};

/// Resolve a sequence of `TemplateValue`s into `ProgramValue`s, consuming
/// parent stack slots and looking up parent labels as needed.
fn resolve_to_program_values(
  source: Vec<TemplateValue>,
  consumed_stack: &mut Vec<Option<DataValue>>,
  parent_scope: &HashMap<String, DataValue>,
  global_scope: &HashMap<String, DataValue>,
) -> Vec<ProgramValue> {
  let mut rendered: Vec<ProgramValue> = Vec::new();
  use TemplateValue as TV;
  for entry in source { match entry {
    TV::Literal(v) => { rendered.push(v); },
    TV::ParentLabel(l) => {
      if let Some(v) = parent_scope.get(&l) {
        rendered.push(v.clone().into())
      } else if let Some(v) = global_scope.get(&l) {
        rendered.push(v.clone().into())
      } else {
        panic!("Undefined label dereferenced: {}", l);
      }
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

/// Extract `DataValue`s from a resolved `ProgramValue` sequence.
/// Panics if any element is `Invoke` or an unrendered `Template`.
fn program_values_to_data_values(program_values: Vec<ProgramValue>) -> Vec<DataValue> {
  program_values.into_iter().map(|pv| match pv {
    ProgramValue::Data(v) => v,
    ProgramValue::Invoke | ProgramValue::ComptimeInvoke => panic!("Invoke token found where a data value was expected"),
    ProgramValue::Template(_) => panic!("Unrendered template found where a data value was expected"),
  }).collect()
}

pub fn render_template(
  template: Template,
  parent_stack: &mut Vec<TemplateValue>,
  parent_scope: &HashMap<String, DataValue>,
  global_scope: &HashMap<String, DataValue>,
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
    TD::Substack(source) => DataValue::Substack(
      resolve_to_program_values(source, &mut consumed_stack, parent_scope, global_scope)
    ),
    TD::Script(source) => DataValue::Script(
      resolve_to_program_values(source, &mut consumed_stack, parent_scope, global_scope)
    ),
    TD::List(source) => {
      let pvs = resolve_to_program_values(source, &mut consumed_stack, parent_scope, global_scope);
      DataValue::List(program_values_to_data_values(pvs))
    },
    TD::Set(source) => {
      let pvs = resolve_to_program_values(source, &mut consumed_stack, parent_scope, global_scope);
      DataValue::Set(program_values_to_data_values(pvs))
    },
    TD::Map(pairs) => {
      let mut entries: Vec<(DataValue, DataValue)> = Vec::new();
      for (key_tv, val_tv) in pairs {
        let key_pvs = resolve_to_program_values(vec![key_tv], &mut consumed_stack, parent_scope, global_scope);
        let val_pvs = resolve_to_program_values(vec![val_tv], &mut consumed_stack, parent_scope, global_scope);
        let key = program_values_to_data_values(key_pvs).into_iter().next().expect("empty map key");
        let val = program_values_to_data_values(val_pvs).into_iter().next().expect("empty map value");
        entries.push((key, val));
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
