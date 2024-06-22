use sid::*;

use std::collections::HashMap;

pub fn render_test_fixture(
  template: Template,
  mut parent_stack: Vec<DataValue>,
  parent_scope: HashMap<String, RealValue>,
  global_scope: HashMap<String, RealValue>,
  expected_parent_stack: Vec<DataValue>,
  expected_rendered_stack: Vec<DataValue>,
) {
  let rendered_stack = render_template(
    template,
    &mut parent_stack,
    &parent_scope,
    &global_scope,
  );
  // Verify remaining parent stack
  assert_eq!(
    parent_stack,
    expected_parent_stack,
    "Parent stack after render wasn't as expected"
  );
  // Verify rendered stack
  assert_eq!(
    rendered_stack,
    expected_rendered_stack,
    "Rendered stack wasn't as expected."
  );
}

#[test]
fn render_empty_substack() {
  render_test_fixture(
    // Template
    Template::substack((
      vec![
      ],
      0
    )),
    // Parent stack
    vec![
    ],
    // Parent and global scope, respectively
    HashMap::new(),
    HashMap::new(),
    // Expected parent stack
    vec![],
    // Expected rendered stack
    vec![
      RealValue::Substack(vec![]).into(),
    ],
  )
}

#[test]
fn render_substack() {
  let mut global = HashMap::new();
  global.insert(
    "one".to_string(),
    RealValue::Int(1),
  );
  render_test_fixture(
    // Template
    Template::substack((
      vec![
        TemplateValue::ParentStackMove(1).into(),
        TemplateValue::ParentLabel("one".to_string()).into(),
        DataValue::Label("add".to_string()).into(),
        ProgramValue::Invoke.into(),
      ],
      1
    )),
    // Parent stack
    vec![
      RealValue::Bool(true).into(),
      RealValue::Int(2).into(),
    ],
    // Parent and global scope, respectively
    HashMap::new(),
    global,
    // Expected parent stack
    vec![RealValue::Bool(true).into()],
    // Expected rendered stack
    vec![
      RealValue::Substack(vec![
        RealValue::Int(2).into(),
        RealValue::Int(1).into(),
        DataValue::Label("add".to_string()).into(),
        ProgramValue::Invoke.into(),
      ]).into(),
    ],
  )
}
