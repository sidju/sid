use super::{Graphemes, parse_program_sequence};
use std::iter::Peekable;
use anyhow::{bail, Result};
use crate::{Template, TemplateData, TemplateValue};

/// Parse a `$n` (parent stack move) or `$name` (parent label) access.
///
/// The iterator must be positioned at the `$`.
pub fn parse_parent_access(input: &mut Peekable<Graphemes>) -> Result<TemplateValue> {
    assert_eq!(input.next(), Some("$"), "parse_parent_access: iterator must start at '$'");
    let mut numeric = true;
    let mut agg = String::new();
    while let Some(&ch) = input.peek() {
        if super::is_key_char(ch) { break; }
        if ch.len() != 1 || !ch.chars().next().unwrap().is_ascii_digit() {
            numeric = false;
        }
        agg.push_str(ch);
        input.next();
    }
    if agg.is_empty() {
        bail!("bare '$' with no index or label name");
    }
    Ok(if numeric {
        TemplateValue::ParentStackMove(agg.parse()?)
    } else {
        TemplateValue::ParentLabel(agg)
    })
}

/// Parse a template literal (`(…)`, `[…]`, `{…}`, or `<…>`).
///
/// For `{…}` the content is scanned at the first parsing level to decide
/// whether it is a set or a struct (struct has at least one `:` at depth 0).
///
/// The iterator must be positioned at the opening delimiter.
pub fn parse_template(input: &mut Peekable<Graphemes>) -> Result<Template> {
    match input.next().as_deref() {
        Some("(") => Ok(Template::substack(parse_program_sequence(input, Some(")"))?)),
        Some("[") => Ok(Template::list(parse_program_sequence(input, Some("]"))?)),
        Some("<") => Ok(Template::script(parse_program_sequence(input, Some(">"))?)),
        Some("{") => parse_brace_template(input),
        other => bail!("expected template opening delimiter, got {:?}", other),
    }
}

/// Disambiguate `{…}` as set vs struct by peeking for a top-level `:`.
fn parse_brace_template(input: &mut Peekable<Graphemes>) -> Result<Template> {
    // Collect all raw graphemes up to the matching `}`, tracking depth so we
    // only look at the first parsing level for `:`.
    let mut buf: Vec<String> = Vec::new();
    let mut depth = 0usize;
    let mut has_colon = false;
    loop {
        match input.next().as_deref() {
            None => bail!("unterminated '{{' — reached end of input"),
            Some("}") if depth == 0 => break,
            Some(ch @ ("(" | "[" | "{" | "<")) => { depth += 1; buf.push(ch.to_owned()); }
            Some(ch @ (")" | "]" | "}" | ">")) => { depth -= 1; buf.push(ch.to_owned()); }
            Some(":") if depth == 0 => { has_colon = true; buf.push(":".to_owned()); }
            Some(ch) => buf.push(ch.to_owned()),
        }
    }
    let raw: String = buf.join("");
    if has_colon {
        parse_struct(&raw)
    } else {
        Ok(Template::set(parse_program_sequence(
            &mut unicode_segmentation::UnicodeSegmentation::graphemes(raw.as_str(), true).peekable(),
            None,
        )?))
    }
}

/// Parse `key: value, …` pairs from the collected brace body.
fn parse_struct(raw: &str) -> Result<Template> {
    use unicode_segmentation::UnicodeSegmentation;
    let mut iter = raw.graphemes(true).peekable();
    let mut pairs: Vec<(TemplateValue, TemplateValue)> = Vec::new();
    let mut max_consumed = 0usize;
    loop {
        // Skip whitespace / commas
        loop {
            match iter.peek() {
                Some(&" ") | Some(&"\n") | Some(&"\t") | Some(&",") => { iter.next(); }
                _ => break,
            }
        }
        if iter.peek().is_none() { break; }
        // Parse key (must be a label)
        let key = super::parse_label(&mut iter)?;
        // Expect `:`
        loop {
            match iter.peek() {
                Some(&" ") | Some(&"\n") | Some(&"\t") => { iter.next(); }
                _ => break,
            }
        }
        match iter.next().as_deref() {
            Some(":") => (),
            other => bail!("expected ':' after struct key, got {:?}", other),
        }
        // Parse value — a single template value
        let (mut seq, consumed) = parse_program_sequence(&mut iter, Some(","))?;
        max_consumed = max_consumed.max(consumed);
        // Reparse remainder if we hit end without a comma terminator
        let val = if seq.len() == 1 {
            seq.remove(0)
        } else {
            bail!("expected exactly one value per struct field, got {}", seq.len())
        };
        pairs.push((key.into(), val));
    }
    Ok(Template {
        data: TemplateData::Struct(pairs),
        consumes_stack_entries: max_consumed,
    })
}
