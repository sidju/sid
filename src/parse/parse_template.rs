use super::{Graphemes, parse_program_sequence, parse_template_value};
use std::iter::Peekable;
use anyhow::{bail, Result};
use crate::{Template, TemplateValue};

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
        parse_map(&raw)
    } else {
        Ok(Template::set(parse_program_sequence(
            &mut unicode_segmentation::UnicodeSegmentation::graphemes(raw.as_str(), true).peekable(),
            None,
        )?))
    }
}

/// Parse `key: value, …` pairs from the collected brace body.
/// Keys are any single TemplateValue (labels are kept as values, not resolved).
fn parse_map(raw: &str) -> Result<Template> {
    use unicode_segmentation::UnicodeSegmentation;
    let mut iter = raw.graphemes(true).peekable();
    let mut pairs: Vec<(TemplateValue, TemplateValue)> = Vec::new();
    let mut max_consumed = 0usize;

    let track = |tv: &TemplateValue, max: &mut usize| {
        if let TemplateValue::ParentStackMove(i) = tv { *max = (*max).max(*i); }
    };

    loop {
        // Parse key — exactly one value
        let key = match parse_template_value(&mut iter)? {
            Some(v) => v,
            None => break,
        };
        track(&key, &mut max_consumed);

        // Skip whitespace, then expect `:`
        loop {
            match iter.peek() {
                Some(&" ") | Some(&"\n") | Some(&"\t") => { iter.next(); }
                _ => break,
            }
        }
        match iter.next().as_deref() {
            Some(":") => (),
            other => bail!("expected ':' after map key, got {:?}", other),
        }

        let val = match parse_template_value(&mut iter)? {
            Some(v) => v,
            None => bail!("expected map value, got end of input"),
        };
        track(&val, &mut max_consumed);

        pairs.push((key, val));

        // Skip whitespace, then expect `,` or end
        loop {
            match iter.peek() {
                Some(&" ") | Some(&"\n") | Some(&"\t") => { iter.next(); }
                _ => break,
            }
        }
        match iter.peek().map(|s| *s) {
            None => break,
            Some(",") => continue,
            Some(other) => bail!("expected ',' or end of map, got {:?}", other),
        }
    }
    Ok(Template::map(pairs, max_consumed))
}
