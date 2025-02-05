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

/// The generic toplevel parser
///
/// Operates on an iterator of grapheme clusters, returns the parsed stack
/// template. (which can be first rendered and then invoked)
pub fn parse_program_sequence<'a,'b> (
  iter: &'a mut Peekable<Graphemes>,
  terminator: Option<&'b str>,
) -> (Vec<TemplateValue>, usize) {
  parse_template_values(iter, terminator, false)
}

pub fn parse_template_values<'a, 'b>(
  iter: &'a mut Peekable<Graphemes>,
  terminator: Option<&'b str>,
  forbid_invoke: bool,
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
    // Otherwise we parse out a template value (allowing invoke)
    match parse_template_value(iter, forbid_invoke) {
      Some(TemplateValue::ParentStackMove(i)) => {
        stack_entries_consumed = stack_entries_consumed.max(i);
        parsed_program.push(TemplateValue::ParentStackMove(i));
      },
      Some(x) => parsed_program.push(x),
      None => {
        // Returned when we run out of input. If we were expecting a terminator
        // that is an error, otherwise cause to return.
        if terminator == None { break; }
        panic!("Ran out of input while looking for terminator {:?}", terminator)
      }
    }
  }
  (parsed_program, stack_entries_consumed)
}

pub fn parse_template_value<'a> (
  iter: &'a mut Peekable<Graphemes>,
  forbid_invoke: bool,
) -> Option<TemplateValue> {
  loop {
    if let Some(val) = iter.peek() { match *val {
      // If these are found it is an error
      // (they should have been caught as terminator in the calling function or
      // were provided in an invalid location)
      ":" | ")" | "]" | "}" => panic!("Bad char found when parsing for value."),
      // Anything written after an # is a comment and should be ignored
      "#" => { while iter.next().unwrap_or("\n") != "\n" {} }
      // Whitespace generally has no significance, but sometimes the sub-parsers
      // may use it to identify the end of their input
      // (We need to take next to not invoke infinitely)
      " " | "\n" | "\t" | "," => { iter.next(); },
      // Value literals
      "\"" => return Some(RealValue::Str(parse_string(iter).unwrap()).into()),
      "'" => return Some(RealValue::Char(parse_char(iter)).into()),
      // Beginning of a template-able object.
      "{" | "[" | "(" => return Some(parse_template(iter).into()),
      // Executing a function, substack or script is done separately from its
      // declaration
      // (We need to take next to not invoke infinitely)
      "!" => if forbid_invoke { panic!("Invoke given when forbidden!") } else {
        iter.next();
        return Some(ProgramValue::Invoke.into());
      },
      // A $ means accessing parent context, which inserts a RealValue directly
      // when constructing a literal.
      "$" => return Some(parse_parent_access(iter)),
      // Parse number if first char is a digit or minus (start of signed number)
      x if x.chars().next().map(|c| c.is_ascii_digit() || c == '-').unwrap_or(false) => {
        return Some(parse_number(iter).into())
      },
      // When it doesn't match a literal we try to resolve it as a label
      // Which also handles if it is a bool
      _ => return Some(parse_label(iter).into()),
    }}
    else {
      return None;
    }
  }
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
    " " | "\n" | "\t" | "," => true,
    "!" | "#" => true,
    ":" | ")" | "]" | "}" => true,
    _ => false,
  }
}
