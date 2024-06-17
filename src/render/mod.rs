use std::collections::HashMap;

use crate::{
  TemplateData,
  TemplateValue,
  DataValue,
  ProgramValue,
  RealValue,
};

pub fn render_template(
  template: TemplateData,
  mut consumed_stack: Vec<DataValue>,
  parent_scope: &HashMap<String, RealValue>,
  global_scope: &HashMap<String, RealValue>,
) -> Vec<DataValue> {
  let mut consumed_stack: Vec<Option<DataValue>> = consumed_stack.drain(..)
    .map(|x| Some(x))
    .collect()
  ;
  use TemplateData as TD;
  let rendered_template: RealValue = match template {
    TD::SubstackTemplate(source) => {
      let mut rendered: Vec<ProgramValue> = Vec::new();
      use TemplateValue as TV;
      for entry in source { match entry {
        TV::Literal(v) => { rendered.push(v); },
        TV::ParentLabel(l) => {
          // Get the value from scope maps. Try parent before global
          if let Some(v) = parent_scope.get(&l) {
            rendered.push(v.clone().into())
          }
          else if let Some(v) = global_scope.get(&l) {
            rendered.push(v.clone().into())
          }
          else { panic!("Undefined label dereferenced"); }
        }
        TV::ParentStackMove(i) => {
          let value = consumed_stack[i].take().expect("Stack value taken twice in template");
          rendered.push(value.into()); 
        },
      }}
      RealValue::Substack(rendered)
    },
  };
  let mut rendered_stack: Vec<DataValue> = consumed_stack.drain(..)
    .filter_map(|x| x)
    .collect()
  ;
  rendered_stack.push(rendered_template.into());
  rendered_stack
}
