use super::*;

pub fn parse_number(
  input: &mut Peekable<Graphemes>,
) -> RealValue {
  let mut float = false;
  let mut agg = String::new();
  loop { if let Some(ch) = input.peek() {
    match *ch {
      "." if float => panic!("Error, two decimal dots in float literal!"),
      "." => { float = true; agg.push('.'); },
      "-" if !agg.is_empty() => panic!(
        "Error, minus sign after first character in number literal!"
      ),
      "0"|"1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"|"-" => { agg.push_str(ch); },
      x if is_key_char(x) => { break; },
      x => panic!("Invalid char {} in number literal!", x),
    }
    // If it was valid input it didn't break the loop, so we progress the iter
    input.next();
  } else { break; } }
  return if float {
    RealValue::Float(agg.parse().unwrap())
  } else {
    RealValue::Int(agg.parse().unwrap())
  };
}
