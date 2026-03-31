use super::{parse_program_sequence, parse_template_value, Graphemes};
use crate::{Template, TemplateValue};
use anyhow::{bail, Result};
use std::iter::Peekable;

/// Parse a `$n` (parent stack move) or `$name` (parent label) access.
///
/// The iterator must be positioned at the `$`.
pub fn parse_parent_access(input: &mut Peekable<Graphemes>) -> Result<TemplateValue> {
    assert_eq!(
        input.next(),
        Some("$"),
        "parse_parent_access: iterator must start at '$'"
    );
    let mut numeric = true;
    let mut agg = String::new();
    while let Some(&ch) = input.peek() {
        if super::is_key_char(ch) {
            break;
        }
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

///
/// For `{…}` the content is scanned at the first parsing level to decide
/// whether it is a set or a struct (struct has at least one `:` at depth 0).
///
/// The iterator must be positioned at the opening delimiter.
pub fn parse_template(input: &mut Peekable<Graphemes>) -> Result<Template> {
    match input.next().as_deref() {
        Some("(") => Ok(Template::substack(parse_program_sequence(
            input,
            Some(")"),
        )?)),
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
            Some(ch @ ("(" | "[" | "{" | "<")) => {
                depth += 1;
                buf.push(ch.to_owned());
            }
            Some(ch @ (")" | "]" | "}" | ">")) => {
                depth -= 1;
                buf.push(ch.to_owned());
            }
            Some(":") if depth == 0 => {
                has_colon = true;
                buf.push(":".to_owned());
            }
            Some(ch) => buf.push(ch.to_owned()),
        }
    }
    let raw: String = buf.join("");
    if has_colon {
        // {:} is the empty-map literal — a lone `:` with no entries.
        if raw.trim() == ":" {
            return Ok(Template::map(vec![], 0));
        }
        parse_map(&raw)
    } else {
        Ok(Template::set(parse_program_sequence(
            &mut unicode_segmentation::UnicodeSegmentation::graphemes(raw.as_str(), true)
                .peekable(),
            None,
        )?))
    }
}

/// Parse `key: value, …` pairs from the collected brace body.
/// Keys are multi-token sequences up to `:` at depth 0.
/// Values are multi-token sequences up to `,` or end at depth 0.
fn parse_map(raw: &str) -> Result<Template> {
    use unicode_segmentation::UnicodeSegmentation;
    let mut iter = raw.graphemes(true).peekable();
    let mut pairs: Vec<(Vec<TemplateValue>, Vec<TemplateValue>)> = Vec::new();
    let mut max_consumed = 0usize;

    let track_seq = |tvs: &[TemplateValue], max: &mut usize| {
        for tv in tvs {
            if let TemplateValue::ParentStackMove(i) = tv {
                *max = (*max).max(*i);
            }
        }
    };

    // Skip leading whitespace
    let skip_ws = |iter: &mut std::iter::Peekable<_>| loop {
        match iter.peek() {
            Some(&" ") | Some(&"\n") | Some(&"\t") => {
                iter.next();
            }
            _ => break,
        }
    };

    loop {
        skip_ws(&mut iter);

        // Check for end of input
        if iter.peek().is_none() {
            break;
        }

        // Parse key tokens until `:` at depth 0
        let mut key_tvs: Vec<TemplateValue> = Vec::new();
        loop {
            skip_ws(&mut iter);
            match iter.peek().map(|s| *s) {
                None => bail!("unexpected end of input while parsing map key"),
                Some(":") => {
                    iter.next();
                    break;
                }
                _ => match parse_template_value(&mut iter)? {
                    Some(v) => key_tvs.push(v),
                    None => bail!("unexpected end of input while parsing map key"),
                },
            }
        }
        if key_tvs.is_empty() {
            bail!("map key expression is empty");
        }
        track_seq(&key_tvs, &mut max_consumed);

        // Parse value tokens until `,` or end at depth 0
        let mut val_tvs: Vec<TemplateValue> = Vec::new();
        loop {
            skip_ws(&mut iter);
            match iter.peek().map(|s| *s) {
                None | Some(",") => break,
                _ => match parse_template_value(&mut iter)? {
                    Some(v) => val_tvs.push(v),
                    None => break,
                },
            }
        }
        if val_tvs.is_empty() {
            bail!("map value expression is empty");
        }
        track_seq(&val_tvs, &mut max_consumed);

        pairs.push((key_tvs, val_tvs));

        // Consume trailing `,` if present
        skip_ws(&mut iter);
        match iter.peek().map(|s| *s) {
            None => break,
            Some(",") => {
                iter.next();
            }
            Some(other) => bail!("expected ',' or end of map, got {:?}", other),
        }
    }
    Ok(Template::map(pairs, max_consumed))
}
