use crate::{
  DataValue,
  RealValue,
};

use super::is_key_char;

/// Parse out a label from the given char iterator.
///
/// The first char should be the first char in the label.
pub fn parse_label<'a>(
  input: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
) -> DataValue {
  //let mut escaped = false;
  let mut data = String::new();
  loop {
    if let Some(c) = input.peek() {
      // Signifies end of label
      if is_key_char(*c) {
        break;
      }
      data.push_str(*c);
    }
    else { break; }
    input.next();
  }
  if data.is_empty() {
    panic!("Parsed empty label. Error in parse_program_sequnece.");
  }
  else if &data == "true" {
    DataValue::Real(RealValue::Bool(true))
  }
  else if &data == "false" {
    DataValue::Real(RealValue::Bool(false))
  }
  else {
    DataValue::Label(data)
  }
}
