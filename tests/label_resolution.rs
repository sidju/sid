use sid::{
  interpret_str,
  RealValue,
};

// Labels are resolved at the last possible moment, unless prefixed by $.
//
//#[test]
//fn string() {
//  assert_eq!(
//    interpret_str("\"Hello, world!\"").unwrap(),
//    vec![RealValue::Str("Hello, world!".to_owned()).into()],
//  )
//}
