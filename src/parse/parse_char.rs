use super::Graphemes;
use anyhow::{bail, Result};
use std::iter::Peekable;

/// Parse a `'…'` character literal.  The iterator must be positioned at the
/// opening `'`.  The contents must be exactly one unicode grapheme cluster.
pub fn parse_char(input: &mut Peekable<Graphemes>) -> Result<String> {
    match input.next() {
        Some("'") => (),
        other => bail!("expected '\\'\\'' to open char literal, got {:?}", other),
    }
    let ch = match input.next() {
        Some(g) => g.to_owned(),
        None => bail!("unterminated char literal: no grapheme after opening quote"),
    };
    match input.next() {
        Some("'") => (),
        other => bail!("expected '\\'\\'' to close char literal, got {:?}", other),
    }
    Ok(ch)
}
