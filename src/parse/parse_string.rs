use super::Graphemes;
use std::iter::Peekable;
use anyhow::{bail, Result};

/// Parse a `"…"` string literal.  The iterator must be positioned at the
/// opening `"`.
pub fn parse_string(input: &mut Peekable<Graphemes>) -> Result<String> {
    match input.next() {
        Some("\"") => (),
        other => bail!("expected '\"' to open string literal, got {:?}", other),
    }
    let mut data = String::new();
    for ch in input {
        match ch {
            "\"" => return Ok(data),
            x => data.push_str(x),
        }
    }
    bail!("unterminated string literal")
}
