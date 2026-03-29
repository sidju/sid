use sid::*;

use std::collections::HashMap;

pub fn render_test_fixture(
  template: Template,
  mut parent_stack: Vec<TemplateValue>,
  parent_scope: HashMap<String, DataValue>,
  mut global_scope: HashMap<String, DataValue>,
  expected_parent_stack: Vec<TemplateValue>,
  expected_rendered_stack: Vec<DataValue>,
) {
  let builtins = get_interpret_builtins();
  let rendered_stack = {
    let mut gs = GlobalState::new(&mut global_scope);
    render_template(
      template,
      &mut parent_stack,
      &parent_scope,
      &mut gs,
      &builtins,
    )
  };
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
      DataValue::Substack { body: vec![], args: None, ret: None }.into(),
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
      DataValue::Substack { body: vec![
        DataValue::Int(2).into(),
        DataValue::Int(1).into(),
        DataValue::Label("add".to_string()).into(),
        ProgramValue::Invoke.into(),
      ], args: None, ret: None }.into(),
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
        DataValue::Str(std::ffi::CString::new("a").unwrap()).into(),
        DataValue::Str(std::ffi::CString::new("b").unwrap()).into(),
      ],
      0
    )),
    vec![],
    HashMap::new(),
    HashMap::new(),
    vec![],
    vec![DataValue::Set(vec![
      DataValue::Str(std::ffi::CString::new("a").unwrap()),
      DataValue::Str(std::ffi::CString::new("b").unwrap()),
    ])],
  )
}

#[test]
fn render_map() {
  render_test_fixture(
    Template::map(
      vec![
        (vec![DataValue::Label("x".to_owned()).into()], vec![DataValue::Int(1).into()]),
        (vec![DataValue::Label("y".to_owned()).into()], vec![DataValue::Int(2).into()]),
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
    vec![DataValue::Script { body: vec![
      ProgramValue::Data(DataValue::Int(42)),
    ], args: None, ret: None }],
  )
}
