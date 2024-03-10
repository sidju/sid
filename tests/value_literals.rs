use sid::{
  interpret_str,
  Value,
};

#[test]
fn string() {
  assert_eq!(
    interpret_str("\"Hello, world!\"").unwrap(),
    vec![Value::Str("Hello, world!".to_owned())],
  )
}
