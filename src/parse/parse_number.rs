use super::{Graphemes, is_key_char};
use std::iter::Peekable;
use anyhow::{bail, Result};
use crate::RealValue;

/// Parse an integer or float literal.
///
/// The iterator must be positioned at the first character of the number
/// (a digit or a leading `-`).
pub fn parse_number(input: &mut Peekable<Graphemes>) -> Result<RealValue> {
    let mut is_float = false;
    let mut agg = String::new();
    loop {
        match input.peek() {
            None => break,
            Some(&ch) => match ch {
                "." if is_float => bail!("two decimal points in float literal"),
                "." => { is_float = true; agg.push('.'); }
                "-" if !agg.is_empty() => bail!("minus sign after first character in number literal"),
                "0"|"1"|"2"|"3"|"4"|"5"|"6"|"7"|"8"|"9"|"-" => agg.push_str(ch),
                x if is_key_char(x) => break,
                x => bail!("unexpected character {:?} in number literal", x),
            },
        }
        input.next();
    }
    Ok(if is_float {
        RealValue::Float(agg.parse()?)
    } else {
        RealValue::Int(agg.parse()?)
    })
}
