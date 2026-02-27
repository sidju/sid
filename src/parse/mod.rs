use unicode_segmentation::{Graphemes, UnicodeSegmentation};
use std::iter::Peekable;
use anyhow::{bail, Result};

use crate::*;

mod parse_string;
mod parse_char;
mod parse_label;
mod parse_number;
mod parse_template;

use parse_string::parse_string;
use parse_char::parse_char;
pub use parse_label::parse_label;
use parse_number::parse_number;
use parse_template::{parse_parent_access, parse_template, parse_comptime_template};

/// Parse a complete source string into a flat sequence of [`TemplateValue`]s
/// plus the number of parent-stack entries the sequence consumes.
///
/// This is the main entry point for the parser.
pub fn parse_str(source: &str) -> Result<(Vec<TemplateValue>, usize)> {
    parse_program_sequence(
        &mut source.graphemes(true).peekable(),
        None,
    )
}

/// Parse a sequence of [`TemplateValue`]s until `terminator` is consumed or
/// input runs out (when `terminator` is `None`).
pub fn parse_program_sequence(
    iter: &mut Peekable<Graphemes>,
    terminator: Option<&str>,
) -> Result<(Vec<TemplateValue>, usize)> {
    let mut out: Vec<TemplateValue> = Vec::new();
    let mut max_consumed = 0usize;
    loop {
        // Check for the terminator first; consume it and stop.
        if terminator == iter.peek().map(|x| *x) {
            iter.next();
            return Ok((out, max_consumed));
        }
        match parse_template_value(iter)? {
            None => {
                if terminator.is_none() {
                    return Ok((out, max_consumed));
                }
                bail!("unexpected end of input while looking for '{}'", terminator.unwrap());
            }
            Some(TemplateValue::ParentStackMove(i)) => {
                max_consumed = max_consumed.max(i);
                out.push(TemplateValue::ParentStackMove(i));
            }
            Some(v) => out.push(v),
        }
    }
}

/// Parse the next single [`TemplateValue`] from the iterator.
///
/// Returns `None` when the iterator is exhausted (signals end of input to
/// the caller rather than an error, since the caller knows whether more input
/// is required).
pub(super) fn parse_template_value(
    iter: &mut Peekable<Graphemes>,
) -> Result<Option<TemplateValue>> {
    loop {
        let ch = match iter.peek() {
            None => return Ok(None),
            Some(&c) => c,
        };
        match ch {
            // These are only valid as terminators consumed by the parent call.
            ")" | "]" | "}" | ">" => {
                bail!("unexpected closing delimiter '{}'", ch)
            }
            // Comment: skip to end of line.
            "#" => { while iter.next().unwrap_or("\n") != "\n" {} }
            // Insignificant whitespace and comma separators.
            " " | "\n" | "\t" | "," => { iter.next(); }
            // String literal.
            "\"" => return Ok(Some(DataValue::Str(parse_string(iter)?).into())),
            // Char literal.
            "'" => return Ok(Some(DataValue::Char(parse_char(iter)?).into())),
            // Template literals: substack, list, set/struct, script.
            "(" | "[" | "{" | "<" => return Ok(Some(parse_template(iter)?.into())),
            // Invoke / comptime-invoke.
            "!" => { iter.next(); return Ok(Some(ProgramValue::Invoke.into())); }
            "@" => {
                iter.next();
                match iter.peek().map(|x| *x) {
                    Some("!") => { iter.next(); return Ok(Some(ProgramValue::ComptimeInvoke.into())); }
                    Some("(" | "[" | "{" | "<") => {
                        return Ok(Some(parse_comptime_template(iter)?.into()));
                    }
                    other => bail!("expected '!' or template delimiter after '@', got {:?}", other),
                }
            }
            // Stack / scope substitution inside a template.
            "$" => return Ok(Some(parse_parent_access(iter)?)),
            // Number (digit or leading minus).
            x if x.chars().next().map(|c| c.is_ascii_digit() || c == '-').unwrap_or(false) => {
                return Ok(Some(parse_number(iter)?.into()));
            }
            // Anything else is a label or boolean.
            _ => return Ok(Some(parse_label(iter)?.into())),
        }
    }
}

/// Characters that delimit tokens (not valid inside a bare label or number).
pub(super) fn is_key_char(ch: &str) -> bool {
    matches!(ch, " " | "\n" | "\t" | "," | "!" | "#" | ":" | ")" | "]" | "}" | ">")
}

#[cfg(test)]
mod tests;
