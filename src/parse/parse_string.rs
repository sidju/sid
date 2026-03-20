use super::Graphemes;
use std::ffi::CString;
use std::iter::Peekable;
use anyhow::{bail, Result};

/// Parse a `"…"` string literal.  The iterator must be positioned at the
/// opening `"`.
///
/// Returns a [`CString`] (null-terminated, no interior NUL bytes) so the value
/// can be used directly in C FFI calls.  A NUL byte inside the literal is a
/// parse error.
pub fn parse_string(input: &mut Peekable<Graphemes>) -> Result<CString> {
    match input.next() {
        Some("\"") => (),
        other => bail!("expected '\"' to open string literal, got {:?}", other),
    }
    let mut data = String::new();
    for ch in input {
        match ch {
            "\"" => {
                return CString::new(data)
                    .map_err(|_| anyhow::anyhow!("string literal contains NUL byte"));
            }
            x => data.push_str(x),
        }
    }
    bail!("unterminated string literal")
}
