use sid::{
  interpret_str,
  Value,
  RealValue,
  invoke::side_effects::mock_side_effector::MockSideEffector,
};

#[test]
fn string() {
  let mut sa = MockSideEffector{calls: Vec::new()};
  assert_eq!(
    interpret_str(
      "\"Hello, world!\"",
      &mut sa,
    ).unwrap(),
    vec![RealValue::Str("Hello, world!".to_owned()).into()],
  );
  assert!(
    sa.calls.is_empty()
  );
}

#[test]
fn label() {
  let mut sa = MockSideEffector{calls: Vec::new()};
  assert_eq!(
    interpret_str(
      "Hello, world",
      &mut sa,
    ).unwrap(),
    vec![
      Value::Label("Hello,".to_owned()),
      Value::Label("world".to_owned()),
    ],
  );
  assert!(
    sa.calls.is_empty()
  );
}

#[test]
fn bool() {
  let mut sa = MockSideEffector{calls: Vec::new()};
  assert_eq!(
    interpret_str(
      "true false",
      &mut sa,
    ).unwrap(),
    vec![
      RealValue::Bool(true).into(),
      RealValue::Bool(false).into(),
    ],
  );
  assert!(
    sa.calls.is_empty()
  );
}

#[test]
fn integer() {
  let mut sa = MockSideEffector{calls: Vec::new()};
  assert_eq!(
    interpret_str(
      "-10 500000",
      &mut sa,
    ).unwrap(),
    vec![RealValue::Int(-10).into(), RealValue::Int(500000).into()],
  );
  assert!(
    sa.calls.is_empty(),
  );
}

#[test]
fn float() {
  let mut sa = MockSideEffector{calls: Vec::new()};
  assert_eq!(
    interpret_str(
      "-10.5 0.66",
      &mut sa,
    ).unwrap(),
    vec![RealValue::Float(-10.5).into(), RealValue::Float(0.66).into()],
  );
  assert!(
    sa.calls.is_empty(),
  );
}
