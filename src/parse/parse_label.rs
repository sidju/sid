/// Parse out a label from the given char iterator.
///
/// The first char should be the first char in the label.
pub fn parse_label(
  input: &mut std::iter::Peekable<impl Iterator<Item = char>>,
) -> String {
  //let mut escaped = false;
  let mut data = String::new();
  loop {
    match input.peek() {
      // Signify end of label
      Some(' ') | Some('!') | None => { break; },
      _ => { data.push(input.next().unwrap()); },
    }
  }
  data
}
