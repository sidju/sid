use std::collections::HashMap;

use crate::{
  Template,
  TemplateData,
  TemplateValue,
  DataValue,
  ProgramValue,
  RealValue,
};

pub fn render_template<'a>(
  template: Template<'a>,
  parent_stack: &mut Vec<DataValue<'a>>,
  parent_scope: &HashMap<String, RealValue<'a>>,
  global_scope: &HashMap<String, RealValue<'a>>,
) -> Vec<DataValue<'a>> {
  if template.consumes_stack_entries > parent_stack.len() {
    panic!("Template consumes more stack entries than there are.");
  }
  let mut consumed_stack = parent_stack
    .drain(parent_stack.len() - template.consumes_stack_entries ..)
    .map(|x| Some(x))
    .collect::<Vec<Option<DataValue>>>()
  ;
  use TemplateData as TD;
  let rendered_template: RealValue = match template.data {
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
          else { panic!("Undefined label dereferenced: {}", l); }
        }
        TV::ParentStackMove(i) => {
          if i == 0 {
            panic!("Parent stack index 0 is the template!");
          }
          let value = consumed_stack[i - 1]
            .take()
            .expect("Stack value taken twice in template")
          ;
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
