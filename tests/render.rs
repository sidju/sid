use sid::*;

use std::collections::HashMap;

pub fn render_test_fixture(
  template: Template,
  mut parent_stack: Vec<TemplateValue>,
  parent_scope: HashMap<String, DataValue>,
  global_scope: HashMap<String, DataValue>,
  expected_parent_stack: Vec<TemplateValue>,
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
      DataValue::Substack(vec![]).into(),
    ],
  )
}

#[test]
fn render_substack() {
  let mut global = HashMap::new();
  global.insert(
    "one".to_string(),
    DataValue::Int(1),
  );
  render_test_fixture(
    // Template
    Template::substack((
      vec![
        TemplateValue::ParentStackMove(1),
        TemplateValue::ParentLabel("one".to_string()),
        DataValue::Label("add".to_string()).into(),
        ProgramValue::Invoke.into(),
      ],
      1
    )),
    // Parent stack
    vec![
      DataValue::Bool(true).into(),
      DataValue::Int(2).into(),
    ],
    // Parent and global scope, respectively
    HashMap::new(),
    global,
    // Expected parent stack
    vec![DataValue::Bool(true).into()],
    // Expected rendered stack
    vec![
      DataValue::Substack(vec![
        DataValue::Int(2).into(),
        DataValue::Int(1).into(),
        DataValue::Label("add".to_string()).into(),
        ProgramValue::Invoke.into(),
      ]).into(),
    ],
  )
}

#[test]
fn render_list() {
  render_test_fixture(
    Template::list((
      vec![
        DataValue::Int(1).into(),
        DataValue::Int(2).into(),
        DataValue::Int(3).into(),
      ],
      0
    )),
    vec![],
    HashMap::new(),
    HashMap::new(),
    vec![],
    vec![DataValue::List(vec![
      DataValue::Int(1),
      DataValue::Int(2),
      DataValue::Int(3),
    ])],
  )
}

#[test]
fn render_set() {
  render_test_fixture(
    Template::set((
      vec![
        DataValue::Str("a".to_owned()).into(),
        DataValue::Str("b".to_owned()).into(),
      ],
      0
    )),
    vec![],
    HashMap::new(),
    HashMap::new(),
    vec![],
    vec![DataValue::Set(vec![
      DataValue::Str("a".to_owned()),
      DataValue::Str("b".to_owned()),
    ])],
  )
}

#[test]
fn render_map() {
  render_test_fixture(
    Template::map(
      vec![
        (DataValue::Label("x".to_owned()).into(), DataValue::Int(1).into()),
        (DataValue::Label("y".to_owned()).into(), DataValue::Int(2).into()),
      ],
      0
    ),
    vec![],
    HashMap::new(),
    HashMap::new(),
    vec![],
    vec![DataValue::Map(vec![
      (DataValue::Label("x".to_owned()), DataValue::Int(1)),
      (DataValue::Label("y".to_owned()), DataValue::Int(2)),
    ])],
  )
}

#[test]
fn render_script() {
  render_test_fixture(
    Template::script((
      vec![
        DataValue::Int(42).into(),
      ],
      0
    )),
    vec![],
    HashMap::new(),
    HashMap::new(),
    vec![],
    vec![DataValue::Script(vec![
      ProgramValue::Data(DataValue::Int(42)),
    ])],
  )
}
