use std::error::Error;
use std::collections::HashMap;

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

mod types;
pub use types::{
  Value,
  RealValue,
  ProgramValue,
  Function,
};

/// The generic parser
///
/// Operates on an iterator of grapheme clusters, returns the parsed program
/// stack. (which can then be invoked)
pub fn parse<'a> (
  source_iter: impl Iterator<Item = &'a str>,
) -> Result<Vec<ProgramValue>, Box<dyn Error>> {
  // State for the interpreter
  let mut parsed_program: Vec<ProgramValue> = Vec::new();
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
      "\"" => parsed_program.push(RealValue::Str(parse_string(&mut iter)?).into()),
      "'" => parsed_program.push(RealValue::Char(parse_char(&mut iter)).into()),
      // Executing a function, substack or script is done separately from its
      // declaration
      // (We need to take next to not invoke infinitely)
      "!" => { iter.next(); parsed_program.push(ProgramValue::Invoke); },
      // A $ means accessing parent context, which inserts a RealValue directly
      // when constructing a literal.
      "$" => todo!(),
      // Parse number if first char is a digit or minus (start of signed number)
      x if x.chars().next().map(|c| c.is_ascii_digit() || c == '-').unwrap_or(false) => {
        parsed_program.push(parse_number(&mut iter).into())
      },
      // When it doesn't match a literal we try to resolve it as a label
      // Which also handles if it is a bool
      _ => parsed_program.push(parse_label(&mut iter).into()),
    }}
    else { break; }
  }
  Ok(parsed_program)
}

pub fn interpret<'a>(
  program: Vec<ProgramValue>,
  side_effector: &mut dyn SideEffector,
  global_scope: HashMap<String, RealValue>,
) -> Result<Vec<Value>, Box<dyn Error>> {
  todo!()
}

pub fn interpret_str(
  script: &str,
  side_effector: &mut dyn SideEffector,
) -> Result<Vec<Value>, Box<dyn Error>> {
  let mut scope = HashMap::new();
  todo!();
  interpret(
    parse_str(script)?,
    side_effector,
    scope,
  )
}

pub fn parse_str(
  script: &str,
) -> Result<Vec<ProgramValue>, Box<dyn Error>> {
  parse(
    script.graphemes(true),
  )
}
