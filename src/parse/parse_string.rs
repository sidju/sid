#[derive(Debug)]
pub enum ParseStringError {
  UnterminatedString,
}
impl std::fmt::Display for ParseStringError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result{
    match self {
      Self::UnterminatedString => {
        write!(f, "String doesn't end before the source code does.")
      },
    }
  }
}
impl std::error::Error for ParseStringError {}

/// Parse out a quoted string from the given char iterator.
///
/// The first char should be the opening quote.
pub fn parse_string(
  input: &mut impl Iterator<Item = char>,
) -> Result<String, ParseStringError> {
  match input.next() {
    Some('"') => (),
    x => panic!("Invalid call to parse_string, first char should be \"."),
  }
  //let mut escaped = false;
  let mut data = String::new();
  for ch in input { match ch {
    '"' => { return Ok(data); },
    x => { data.push(x); },
  } }
  // Long term we want to include when the string started, but that's later
  // when we add that context data to the function input
  Err(ParseStringError::UnterminatedString)
}
