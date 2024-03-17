use super::RealValue;

pub fn parse_number(
  input: &mut impl Iterator<Item = char>,
) -> RealValue {
  let mut float = false;
  let mut signed = false;
  let mut agg = String::new();
  for ch in input { match ch {
    ' ' => { break; },
    '.' if float => panic!("Error, two decimal dots in float literal!"),
    '.' => { float = true; agg.push('.'); },
    '-' if signed => panic!("Error, two decimal dots in float literal!"),
    '-' => { signed = true; agg.push('-'); },
    x if x.is_ascii_digit() => { agg.push(x); },
    _ => panic!("Invalid characters in number literal!"),
  } }
  return if float {
    RealValue::Float(agg.parse().unwrap())
  } else {
    RealValue::Int(agg.parse().unwrap())
  };
}
