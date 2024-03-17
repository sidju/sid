use crate::{
  Value,
  RealValue,
};

/// Parse out a label from the given char iterator.
///
/// The first char should be the first char in the label.
pub fn parse_label<'a>(
  input: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
) -> Value {
  //let mut escaped = false;
  let mut data = String::new();
  loop {
    match input.peek() {
      // Signify end of label
      Some(&" ") | Some(&"!") | None => { break; },
      _ => { data.push_str(input.next().unwrap()); },
    }
  }
  if &data == "true" {
    Value::Real(RealValue::Bool(true))
  }
  else if &data == "false" {
    Value::Real(RealValue::Bool(false))
  }
  else {
    Value::Label(data)
  }
}
