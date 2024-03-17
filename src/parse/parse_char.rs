// You'd think this would return a char, but since a char in the language should
// be a unicode grapheme (simplified that's what's printed as one symbol)
pub fn parse_char<'a>(
  input: &mut impl Iterator<Item = &'a str>,
) -> String {
  match input.next() {
    Some("'") => (),
    _ => panic!("Invalid call to parse_char, input should start with '"),
  }
  let ch = input.next().expect("No character after starting quote of character literal.");
  match input.next() {
    Some("'") => (),
    _ => panic!("character literal incorrectly terminated")
  }
  ch.to_owned()
}
