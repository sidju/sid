use super::{Graphemes, is_key_char};
use std::iter::Peekable;
use anyhow::{bail, Result};
use crate::DataValue;

/// Parse a label or boolean literal.
///
/// A label is any sequence of graphemes that are not key characters.
/// `true` and `false` are recognised as boolean literals.
pub fn parse_label(input: &mut Peekable<Graphemes>) -> Result<DataValue> {
    let mut data = String::new();
    loop {
        match input.peek() {
            None => break,
            Some(&ch) if is_key_char(ch) => break,
            Some(&ch) => data.push_str(ch),
        }
        input.next();
    }
    if data.is_empty() {
        bail!("expected a label but found a key character or end of input");
    }
    Ok(match data.as_str() {
        "true"  => DataValue::Bool(true),
        "false" => DataValue::Bool(false),
        _       => DataValue::Label(data),
    })
}
