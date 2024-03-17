use sid::{
  interpret_str,
  Value,
  RealValue,
  invoke::side_effects::{
    SideEffectFunction,
    mock_side_effector::{
      MockSideEffector,
      MockCall,
    },
  },
};

#[test]
fn print() {
  let mut sa = MockSideEffector{calls: Vec::new()};
  let stack = interpret_str(
    "\"Hello, world!\" print!",
    &mut sa,
  ).unwrap();
  assert_eq!(
    sa.calls,
    vec![
      MockCall{
        func: SideEffectFunction::Print, args: vec![
          RealValue::Str("Hello, world!".to_owned()).into(),
        ]
      }
    ],
  );
  assert_eq!(
    stack,
    vec![],
  );
}
