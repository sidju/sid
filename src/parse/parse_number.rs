use super::RealValue;

pub fn parse_number<'a>(
  input: &mut impl Iterator<Item = &'a str>,
) -> RealValue {
  let mut float = false;
  let mut signed = false;
  let mut agg = String::new();
  for ch in input { match ch {
    " " => { break; },
    "." if float => panic!("Error, two decimal dots in float literal!"),
    "." => { float = true; agg.push('.'); },
    "-" if signed => panic!("Error, two decimal dots in float literal!"),
    "-" => { signed = true; agg.push('-'); },
    "0"|"1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9" => { agg.push_str(ch); },
    _ => panic!("Invalid characters in number literal!"),
  } }
  return if float {
    RealValue::Float(agg.parse().unwrap())
  } else {
    RealValue::Int(agg.parse().unwrap())
  };
}
