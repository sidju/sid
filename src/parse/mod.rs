use std::iter::Peekable;

use super::*;

use unicode_segmentation::{UnicodeSegmentation, Graphemes};

mod parse_string;
pub use parse_string::*;

mod parse_char;
pub use parse_char::*;

mod parse_label;
pub use parse_label::*;

mod parse_number;
pub use parse_number::*;

mod parse_template;
pub use parse_template::*;

/// The generic parser
///
/// Operates on an iterator of grapheme clusters, returns the parsed stack
/// template. (which can be first rendered and then invoked)
pub fn parse_program_sequence<'a,'b> (
  iter: &'a mut Peekable<Graphemes>,
  terminator: Option<&'b str>,
) -> (Vec<TemplateValue>, usize) {
  // State for the interpreter
  let mut parsed_program: Vec<TemplateValue> = Vec::new();
  let mut stack_entries_consumed = 0;
  loop {
    // Check for the terminator. If found we consume it, break and return
    if terminator == iter.peek().map(|x|*x) {
      iter.next();
      break;
    }
    if let Some(val) = iter.peek() { match *val {
      // Whitespace generally has no significance, but sometimes the sub-parsers
      // may use it to identify the end of their input
      // (We need to take next to not invoke infinitely)
      " " => { iter.next(); },
      // Value literals
      "\"" => parsed_program.push(RealValue::Str(parse_string(iter).unwrap()).into()),
      "'" => parsed_program.push(RealValue::Char(parse_char(iter)).into()),
      // Beginning of a template-able object.
      "{" | "[" | "(" => parsed_program.push(parse_template(iter).into()),
      // Executing a function, substack or script is done separately from its
      // declaration
      // (We need to take next to not invoke infinitely)
      "!" => { iter.next(); parsed_program.push(ProgramValue::Invoke.into()); },
      // A $ means accessing parent context, which inserts a RealValue directly
      // when constructing a literal.
      "$" => match parse_parent_access(iter) {
        TemplateValue::ParentStackMove(i) => {
          stack_entries_consumed = stack_entries_consumed.max(i); 
          parsed_program.push(TemplateValue::ParentStackMove(i));
        },
        x => parsed_program.push(x),
      },
      // Parse number if first char is a digit or minus (start of signed number)
      x if x.chars().next().map(|c| c.is_ascii_digit() || c == '-').unwrap_or(false) => {
        parsed_program.push(parse_number(iter).into())
      },
      // When it doesn't match a literal we try to resolve it as a label
      // Which also handles if it is a bool
      _ => parsed_program.push(parse_label(iter).into()),
    }}
    else {
      panic!("Ran out of input while looking for terminator {:?}", terminator)
    }
  }
  (parsed_program, stack_entries_consumed)
}

pub fn parse_str(
  script: &str,
) -> (Vec<TemplateValue>, usize) {
  parse_program_sequence(
    &mut script.graphemes(true).peekable(),
    None,
  )
}

fn is_key_char(
  ch: &str,
) -> bool {
  match ch {
    " " | "!" | ")" | "\"" | "'" | "]" => true,
    _ => false,
  }
}
