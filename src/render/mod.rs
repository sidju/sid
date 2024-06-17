pub fn render_template(
  template: Template,
  mut consumed_stack: Vec<Value>,
  parent_scope: &HashMap<String, RealValue>,
  global_scope: &HashMap<String, RealValue>,
) -> DataValue {
  let mut consumed_stack: Vec<Option<RealValue>> = consumed_stack.drain(..).map(
    |x| Some(realize_value(x))
  ).collect();
  use Template as T;
  match template {
    T::SubstackTemplate(source) => {
      let mut rendered: Vec<ProgramValue> = Vec::new();
      use TemplateValue as TV;
      for entry in source { match entry {
        TV::Literal(v) => { rendered.push(v); },
        TV::ParentLabel(l) => { rendered.push(resolve_label(&l).into()); },
        TV::ParentStackMove(i) => {
          let value = consumed_stack[i].take().expect("Stack value taken twice in template");
          rendered.push(value.into()); 
        },
      }}
      RealValue::Substack(rendered)
    },
  }
}
