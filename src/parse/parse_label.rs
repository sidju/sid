/// Parse out a label from the given char iterator.
///
/// The first char should be the first char in the label.
pub fn parse_label(
  input: &mut impl Iterator<Item = char>,
) -> String {
  //let mut escaped = false;
  let mut data = String::new();
  for ch in input { match ch {
    ' ' => { return data; },
    x => { data.push(x); },
  } }
  data
}
