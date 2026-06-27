use std::ops::Range;

use indexmap::IndexMap;

use super::{
    RawBlock, RawDocumentData, RawEntryData, RawFieldData, RawValueMode, is_safe_bare_value,
    is_valid_entry_key, is_valid_identifier,
};
use crate::quoted;

type ParsedValue = (String, usize, Option<Range<usize>>, RawValueMode);
pub fn parse_raw_document(source: &str) -> RawDocumentData {
    let mut blocks = Vec::new();
    let mut entries: IndexMap<String, Vec<usize>> = IndexMap::new();
    let mut entry_blocks = Vec::new();
    let mut pos = 0;
    while pos < source.len() {
        let Some(ch) = source[pos..].chars().next() else {
            break;
        };

        if ch.is_whitespace() {
            let end = take_while(source, pos, char::is_whitespace);
            blocks.push(RawBlock::Whitespace {
                raw: source[pos..end].to_string(),
                span: pos..end,
            });
            pos = end;
            continue;
        }

        if ch == '%' {
            let end = take_line(source, pos);
            blocks.push(RawBlock::Comment {
                raw: source[pos..end].to_string(),
                span: pos..end,
            });
            pos = end;
            continue;
        }

        if ch != '@' {
            let end = source[pos + ch.len_utf8()..]
                .find(['@', '%'])
                .map(|offset| pos + ch.len_utf8() + offset)
                .unwrap_or(source.len());
            blocks.push(RawBlock::Other {
                raw: source[pos..end].to_string(),
                span: pos..end,
            });
            pos = end;
            continue;
        }

        let (mut block, parsed_entry, end) = parse_at_block(source, pos);
        if let Some(entry) = parsed_entry {
            let id = entry_blocks.len();
            if let RawBlock::Entry { id: block_id, .. } = &mut block {
                *block_id = id;
            }
            entries.entry(entry.key.clone()).or_default().push(id);
            entry_blocks.push(entry);
        }
        blocks.push(block);
        pos = end;
    }

    RawDocumentData {
        blocks,
        entries,
        entry_blocks,
    }
}

fn parse_at_block(source: &str, start: usize) -> (RawBlock, Option<RawEntryData>, usize) {
    match find_at_block_end(source, start) {
        Ok(end) => parse_complete_at_block(source, start, end),
        Err((end, error)) => (
            RawBlock::Failed {
                raw: source[start..end].to_string(),
                error,
                span: start..end,
            },
            None,
            end,
        ),
    }
}

fn parse_complete_at_block(
    source: &str,
    start: usize,
    end: usize,
) -> (RawBlock, Option<RawEntryData>, usize) {
    let raw = &source[start..end];
    let body_start = raw.find(['{', '(']).unwrap_or(raw.len());
    let kind = raw[1..body_start].trim().to_ascii_lowercase();
    let absolute_body_start = start + body_start + 1;
    let absolute_body_end = end.saturating_sub(1);
    let body = &source[absolute_body_start..absolute_body_end];

    if !is_valid_identifier(&kind) {
        return (
            RawBlock::Failed {
                raw: raw.to_string(),
                error: format!("entry type {} is invalid", quoted(&kind)),
                span: start..end,
            },
            None,
            end,
        );
    }

    match kind.as_str() {
        "comment" => (
            RawBlock::Comment {
                raw: raw.to_string(),
                span: start..end,
            },
            None,
            end,
        ),
        "preamble" => (
            RawBlock::Preamble {
                raw: raw.to_string(),
                value: parse_preamble_value(body),
                span: start..end,
            },
            None,
            end,
        ),
        "string" => {
            if let Some((key, value)) = parse_assignment(body) {
                (
                    RawBlock::StringDef {
                        raw: raw.to_string(),
                        key,
                        value,
                        span: start..end,
                    },
                    None,
                    end,
                )
            } else {
                (
                    RawBlock::Failed {
                        raw: raw.to_string(),
                        error: "string definition is missing '='".to_string(),
                        span: start..end,
                    },
                    None,
                    end,
                )
            }
        }
        _ => match parse_entry(source, start, end, &kind, absolute_body_start, body) {
            Ok(entry) => {
                let key = entry.key.clone();
                (
                    RawBlock::Entry {
                        id: usize::MAX,
                        key,
                        span: start..end,
                    },
                    Some(entry),
                    end,
                )
            }
            Err(error) => (
                RawBlock::Failed {
                    raw: raw.to_string(),
                    error,
                    span: start..end,
                },
                None,
                end,
            ),
        },
    }
}

fn parse_entry(
    source: &str,
    start: usize,
    end: usize,
    kind: &str,
    body_start: usize,
    body: &str,
) -> Result<RawEntryData, String> {
    let comma = body
        .find(',')
        .ok_or_else(|| "entry key is missing".to_string())?;
    let key = body[..comma].trim().to_string();
    if key.is_empty() {
        return Err("entry key is empty".to_string());
    }
    if !is_valid_entry_key(&key) {
        return Err(format!("entry key {} is invalid", quoted(&key)));
    }

    let mut fields: IndexMap<String, Vec<usize>> = IndexMap::new();
    let mut field_blocks = Vec::new();
    let mut cursor = comma + 1;
    while cursor < body.len() {
        skip_field_gap(body, &mut cursor);
        if cursor >= body.len() {
            break;
        }

        let name_start = cursor;
        while cursor < body.len() {
            let ch = body[cursor..].chars().next().unwrap();
            if ch == '=' || ch.is_whitespace() {
                break;
            }
            cursor += ch.len_utf8();
        }
        let name = body[name_start..cursor].trim().to_ascii_lowercase();
        if name.is_empty() {
            return Err("field name is empty".to_string());
        }
        if !is_valid_identifier(&name) {
            return Err(format!("field name {} is invalid", quoted(&name)));
        }

        while cursor < body.len() && body[cursor..].chars().next().unwrap().is_whitespace() {
            cursor += body[cursor..].chars().next().unwrap().len_utf8();
        }
        if !body[cursor..].starts_with('=') {
            return Err(format!("field {name} is missing '='"));
        }
        cursor += 1;
        while cursor < body.len() && body[cursor..].chars().next().unwrap().is_whitespace() {
            cursor += body[cursor..].chars().next().unwrap().len_utf8();
        }

        let value_start = cursor;
        let (value, value_end, inner_span, value_mode) = parse_value(body, cursor, body_start)?;
        cursor = value_end;
        let field_id = field_blocks.len();
        fields.entry(name.clone()).or_default().push(field_id);
        field_blocks.push(RawFieldData {
            name: name.clone(),
            value,
            value_mode,
            span: inner_span.unwrap_or((body_start + value_start)..(body_start + value_end)),
            patch_span: (body_start + value_start)..(body_start + value_end),
            changed: false,
        });

        skip_field_trivia(body, &mut cursor);
        if cursor < body.len() {
            let ch = body[cursor..].chars().next().unwrap();
            if ch != ',' {
                return Err(format!("field {name} is missing a separator"));
            }
            cursor += ch.len_utf8();
        }
    }

    Ok(RawEntryData {
        key,
        kind: kind.to_string(),
        fields,
        field_blocks,
        span: start..end,
        raw: source[start..end].to_string(),
    })
}

fn find_at_block_end(source: &str, start: usize) -> Result<usize, (usize, String)> {
    let next_block = find_recovery_block_start(source, start);
    let Some(open_rel) = source[start..].find(['{', '(']) else {
        return Err((
            next_block.unwrap_or_else(|| take_line(source, start)),
            "entry opener is missing".to_string(),
        ));
    };
    let open = start + open_rel;
    if let Some(next) = next_block
        && next < open
    {
        return Err((next, "entry opener is missing".to_string()));
    }
    let opener = source[open..].chars().next().unwrap();
    let root_closer = if opener == '{' { '}' } else { ')' };
    let kind = source[start + 1..open].trim().to_ascii_lowercase();
    let raw_comment = kind == "comment";
    let mut in_entry_key =
        is_valid_identifier(&kind) && !matches!(kind.as_str(), "comment" | "preamble" | "string");
    let mut closers = vec![root_closer];
    let mut in_quote = false;
    let mut quote_brace_depth = 0usize;
    let mut escaped = false;
    let mut pos = open + opener.len_utf8();

    while pos < source.len() {
        let ch = source[pos..].chars().next().unwrap();
        if in_entry_key {
            if ch == ',' {
                in_entry_key = false;
            } else if ch == root_closer {
                return Ok(pos + ch.len_utf8());
            }
            pos += ch.len_utf8();
            continue;
        }
        if in_quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '{' {
                quote_brace_depth += 1;
            } else if ch == '}' && quote_brace_depth > 0 {
                quote_brace_depth -= 1;
            } else if ch == '"' && quote_brace_depth == 0 {
                in_quote = false;
            }
        } else if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '%' && closers.len() == 1 && !raw_comment {
            pos = take_line(source, pos);
            continue;
        } else if ch == '"' && closers.len() == 1 && !raw_comment {
            in_quote = true;
            quote_brace_depth = 0;
        } else if ch == '{' {
            closers.push('}');
        } else if ch == '(' && closers.last() == Some(&')') {
            closers.push(')');
        } else if closers.last() == Some(&ch) {
            closers.pop();
            if closers.is_empty() {
                return Ok(pos + ch.len_utf8());
            }
        }
        pos += ch.len_utf8();
    }

    Err((
        next_block.unwrap_or(source.len()),
        "entry ended before closing delimiter".to_string(),
    ))
}

fn find_recovery_block_start(source: &str, start: usize) -> Option<usize> {
    let mut cursor = take_line(source, start);
    while cursor < source.len() {
        let line_start = cursor;
        while cursor < source.len() {
            let ch = source[cursor..].chars().next().unwrap();
            if !ch.is_whitespace() || ch == '\n' {
                break;
            }
            cursor += ch.len_utf8();
        }
        if source[cursor..].starts_with('@') {
            return Some(cursor);
        }
        cursor = take_line(source, line_start);
    }
    None
}

pub(super) fn parse_value(
    body: &str,
    start: usize,
    body_offset: usize,
) -> Result<ParsedValue, String> {
    let first = parse_value_atom(body, start, body_offset)?;
    let mut cursor = first.1;
    let mut expression_cursor = cursor;
    skip_field_space(body, &mut expression_cursor);
    if !body[expression_cursor..].starts_with('#') {
        return Ok(first);
    }

    while expression_cursor < body.len() && body[expression_cursor..].starts_with('#') {
        expression_cursor += 1;
        skip_field_space(body, &mut expression_cursor);
        let (_, atom_end, _, _) = parse_value_atom(body, expression_cursor, body_offset)?;
        cursor = atom_end;
        expression_cursor = atom_end;
        skip_field_space(body, &mut expression_cursor);
    }

    Ok((
        body[start..cursor].trim().to_string(),
        cursor,
        Some((body_offset + start)..(body_offset + cursor)),
        RawValueMode::Expression,
    ))
}

fn parse_value_atom(body: &str, start: usize, body_offset: usize) -> Result<ParsedValue, String> {
    let Some(ch) = body[start..].chars().next() else {
        return Err("field value is missing".to_string());
    };

    if ch == '{' {
        let end = find_balanced_in_body(body, start, '{', '}')?;
        let inner = (body_offset + start + 1)..(body_offset + end - 1);
        return Ok((
            body[start + 1..end - 1].trim().to_string(),
            end,
            Some(inner),
            RawValueMode::Braced,
        ));
    }

    if ch == '"' {
        let end = find_quoted_end(body, start)?;
        let inner = (body_offset + start + 1)..(body_offset + end - 1);
        return Ok((
            body[start + 1..end - 1].trim().to_string(),
            end,
            Some(inner),
            RawValueMode::Quoted,
        ));
    }

    let mut cursor = start;
    while cursor < body.len() {
        let ch = body[cursor..].chars().next().unwrap();
        if ch.is_whitespace() || ch == ',' || ch == '#' || ch == '%' {
            break;
        }
        cursor += ch.len_utf8();
    }
    if cursor == start {
        return Err("field value is missing".to_string());
    }
    let value = body[start..cursor].trim().to_string();
    if !is_safe_bare_value(&value) {
        return Err(format!("bare field value {} is invalid", quoted(&value)));
    }
    Ok((value, cursor, None, RawValueMode::Bare))
}

fn find_balanced_in_body(
    body: &str,
    start: usize,
    opener: char,
    closer: char,
) -> Result<usize, String> {
    let mut depth = 0usize;
    let mut escaped = false;
    let mut cursor = start;
    while cursor < body.len() {
        let ch = body[cursor..].chars().next().unwrap();
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == opener {
            depth += 1;
        } else if ch == closer {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Ok(cursor + ch.len_utf8());
            }
        }
        cursor += ch.len_utf8();
    }
    Err("field value ended before closing brace".to_string())
}

fn find_quoted_end(body: &str, start: usize) -> Result<usize, String> {
    let mut cursor = start + 1;
    let mut escaped = false;
    let mut brace_depth = 0usize;
    while cursor < body.len() {
        let ch = body[cursor..].chars().next().unwrap();
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '{' {
            brace_depth += 1;
        } else if ch == '}' && brace_depth > 0 {
            brace_depth -= 1;
        } else if ch == '"' && brace_depth == 0 {
            return Ok(cursor + 1);
        }
        cursor += ch.len_utf8();
    }
    Err("field value ended before closing quote".to_string())
}

fn skip_field_gap(body: &str, cursor: &mut usize) {
    while *cursor < body.len() {
        let ch = body[*cursor..].chars().next().unwrap();
        if ch.is_whitespace() || ch == ',' {
            *cursor += ch.len_utf8();
            continue;
        }
        if ch == '%' {
            *cursor = take_line(body, *cursor);
            continue;
        }
        break;
    }
}

fn skip_field_space(body: &str, cursor: &mut usize) {
    while *cursor < body.len() {
        let ch = body[*cursor..].chars().next().unwrap();
        if !ch.is_whitespace() {
            break;
        }
        *cursor += ch.len_utf8();
    }
}

fn skip_field_trivia(body: &str, cursor: &mut usize) {
    loop {
        skip_field_space(body, cursor);
        if *cursor < body.len() && body[*cursor..].starts_with('%') {
            *cursor = take_line(body, *cursor);
            continue;
        }
        break;
    }
}

fn parse_assignment(body: &str) -> Option<(String, String)> {
    let equals = body.find('=')?;
    let key = body[..equals].trim().to_ascii_lowercase();
    if !is_valid_identifier(&key) {
        return None;
    }
    let mut cursor = equals + 1;
    skip_field_space(body, &mut cursor);
    let (value, end, _, _) = parse_value(body, cursor, 0).ok()?;
    cursor = end;
    skip_field_trivia(body, &mut cursor);
    if cursor != body.len() {
        return None;
    }
    Some((key, value))
}

fn parse_preamble_value(body: &str) -> String {
    let mut cursor = 0;
    skip_field_trivia(body, &mut cursor);
    if let Ok((value, end, _, RawValueMode::Braced | RawValueMode::Quoted)) =
        parse_value(body, cursor, 0)
    {
        cursor = end;
        skip_field_trivia(body, &mut cursor);
        if cursor == body.len() {
            return value;
        }
    }
    body.trim().to_string()
}
fn take_while(source: &str, start: usize, predicate: impl Fn(char) -> bool) -> usize {
    let mut cursor = start;
    while cursor < source.len() {
        let ch = source[cursor..].chars().next().unwrap();
        if !predicate(ch) {
            break;
        }
        cursor += ch.len_utf8();
    }
    cursor
}

fn take_line(source: &str, start: usize) -> usize {
    source[start..]
        .find('\n')
        .map(|offset| start + offset + 1)
        .unwrap_or(source.len())
}
