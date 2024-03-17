use std::error::Error;

use unicode_segmentation::UnicodeSegmentation;

// We create side-effects through a trait implementation
// (This allows mocking all side effects in one for testing)
mod parse;
use parse::*;
pub mod invoke;
use invoke::{
  invoke,
  side_effects::{
    SideEffector,
    SideEffectFunction,
  },
};

#[derive(PartialEq, Debug)]
pub enum Function {
  SideEffect(SideEffectFunction),
//  BuiltIn(BuiltInFunction),
}

#[derive(PartialEq, Debug)]
pub enum RealValue {
  Bool(bool),
  Str(String),
  Char(String), // Holds a full grapheme cluster, which requires a string
  Int(i64),
  Float(f64),
  Fun(Function),
}

#[derive(PartialEq, Debug)]
pub enum Value {
  Real(RealValue),
  Label(String),
}
impl From<RealValue> for Value {
  fn from(item: RealValue) -> Self {
    Self::Real(item)
  }
}
impl From<SideEffectFunction> for RealValue {
  fn from(item: SideEffectFunction) -> Self {
    RealValue::Fun(Function::SideEffect(item))
  }
}

/// The generic interpreter
///
/// Operates on an iterator of grapheme clusters, returns the state of the
/// data_stack when it runs out of input.
pub fn interpret<'a>(
  source_iter: impl Iterator<Item = &'a str>,
  side_effector: &mut dyn SideEffector,
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
      // (We need to take next to not invoke infinitely)
      " " => { iter.next(); },
      // Value literals
      "\"" => data_stack.push(RealValue::Str(parse_string(&mut iter)?).into()),
      "'" => data_stack.push(RealValue::Char(parse_char(&mut iter)).into()),
      // Executing a function, substack or script is done separately from its
      // declaration
      // (We need to take next to not invoke infinitely)
      "!" => { iter.next(); invoke(side_effector, &mut data_stack) },
      // A $ means accessing parent context, which inserts a RealValue directly
      // when constructing a literal.
      "$" => todo!(),
      // Parse number if first char is a digit or minus (start of signed number)
      x if x.chars().next().map(|c| c.is_ascii_digit() || c == '-').unwrap_or(false) => {
        data_stack.push(parse_number(&mut iter).into())
      },
      // When it doesn't match a literal we try to resolve it as a label
      // Which also handles if it is a bool
      _ => data_stack.push(parse_label(&mut iter)),
    }}
    else { break; }
  }
  Ok(data_stack)
}

pub fn interpret_str(
  script: &str,
  side_effector: &mut dyn SideEffector,
) -> Result<Vec<Value>, Box<dyn Error>> {
  interpret(
    script.graphemes(true),
    side_effector,
  )
}
