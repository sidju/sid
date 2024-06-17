use sid::{
  interpret_str,
  Value,
  RealValue,
  ProgramValue,
  invoke::side_effects::mock_side_effector::MockSideEffector,
};

#[test]
fn substack() {
  let mut sa = MockSideEffector{calls: Vec::new()};
  let stack = interpret_str(
    "1 2 3 ($1 $2 $3)",
    &mut sa,
  ).unwrap();
  assert!(sa.calls.is_empty());
  assert_eq!(
    stack,
    vec![RealValue::Substack(vec![
      RealValue::Int(3).into(),
      RealValue::Int(2).into(),
      RealValue::Int(1).into(),
    ]).into()]
  );
}
