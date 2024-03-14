use sid::{
  interpret_str,
  Value,
  RealValue,
};

#[test]
fn string() {
  assert_eq!(
    interpret_str("\"Hello, world!\"").unwrap(),
    vec![RealValue::Str("Hello, world!".to_owned()).into()],
  )
}

#[test]
fn label() {
  assert_eq!(
    interpret_str("Hello, world").unwrap(),
    vec![
      Value::Label("Hello,".to_owned()),
      Value::Label("world".to_owned()),
    ],
  )
}
