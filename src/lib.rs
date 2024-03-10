use std::error::Error;

// We create side-effects through a trait implementation
// (This allows mocking all side effects in one for testing)
mod parse;
//mod invoke;

#[derive(PartialEq, Debug)]
pub enum Function {
//  BuiltIn(BuiltInFunction),
}

#[derive(PartialEq, Debug)]
pub enum Value {
  Str(String),
  Fun(Function),
}

/// The generic interpreter
///
/// Operates on an iterator of char, returns the state of the data_stack when
/// it runs out of input.
pub fn interpret(
  source_iter: impl Iterator<Item = char>
) -> Result<Vec<Value>, Box<dyn Error>> {
  // State for the interpreter
  let mut data_stack: Vec<Value> = Vec::new();
  // Make the iterator peekable and then peek to choose which parsing function
  // to call into.
  let mut iter = source_iter.peekable();
  loop {
    if let Some(val) = iter.peek() { match *val {
      // Whitespace generally has no significance, but sometimes the sub-parsers
      // may use it to identify the end of their input
      ' ' => (),
      // Value literals
      '"' => data_stack.push(Value::Str(parse::parse_string(&mut iter)?)),
      // Executing a function, substack or script is done separately from its
      // declaration
//      '!' => invoke(&mut data_stack)?,
      // When it doesn't match a literal we try to resolve it as a label
//      _ => data_stack.push(resolve_label(parse_label(&mut iter))?),
      _ => panic!("Unhandled syntax")
    }}
    else { break; }
  }
  Ok(data_stack)
}

pub fn interpret_str(script: &str) -> Result<Vec<Value>, Box<dyn Error>> {
  interpret(script.chars())
}
