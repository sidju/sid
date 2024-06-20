use super::*;

pub fn parse_parent_access(
  source_iter: &mut Peekable<Graphemes>,
) -> TemplateValue {
  assert_eq!(
    source_iter.next(),
    Some("$"),
    "Incorrect usage. First character in parse_parent_access wasn't $!"
  );
  let mut numeric = true;
  let mut agg = String::new();
  loop { if let Some(ch) = source_iter.peek() {
    // If this character is a key part of the syntax that has priority
    if is_key_char(*ch) { break; }
    // Otherwise check if it's numeric
    if ch.len() != 1 || !ch.chars().next().unwrap().is_ascii_digit() {
      numeric = false;
    }
    agg.push_str(ch);
    // Progress the iterator last, to correctly progress the parsing 
    source_iter.next();
  } else { break; } }
  return if numeric {
    TemplateValue::ParentStackMove(agg.parse().unwrap())
  } else {
    TemplateValue::ParentLabel(agg)
  }
}

pub fn parse_template(
  source_iter: &mut Peekable<Graphemes>,
) -> Template {
  if let Some(val) = source_iter.next() { match val {
    "(" => {
      return Template::substack(parse_program_sequence(
        source_iter,
        Some(")"),
      ));
    },
    _ => panic!("Invalid template initiator, {}", val),
  } }
  else {
    panic!("parse_template should never be called with empty iterator")
  }
}
