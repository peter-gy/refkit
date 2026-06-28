use std::ops::Range;

use indexmap::IndexMap;

use super::{
    RawBlock, RawDocumentData, RawEntryData, RawFieldData, RawValueAtom, RawValueMode,
    is_safe_bare_value, is_valid_entry_key, is_valid_field_name_char, is_valid_identifier,
};
use crate::quoted;

type ParsedValue = (
    String,
    usize,
    Option<Range<usize>>,
    RawValueMode,
    Vec<RawValueAtom>,
);
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
            let end = take_comment(source, pos);
            blocks.push(RawBlock::Comment {
                raw: source[pos..end].to_string(),
                span: pos..end,
            });
            pos = end;
            continue;
        }

        if ch != '@' {
            let end = find_next_root_break(source, pos + ch.len_utf8()).unwrap_or(source.len());
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
        Err((end, error))
            if error == "entry opener is missing" && is_line_style_comment(source, start) =>
        {
            (
                RawBlock::Comment {
                    raw: source[start..end].to_string(),
                    span: start..end,
                },
                None,
                end,
            )
        }
        Err((end, error))
            if error == "entry opener is missing"
                && end < source.len()
                && is_parseable_at_start(source, end) =>
        {
            (
                RawBlock::Other {
                    raw: source[start..end].to_string(),
                    span: start..end,
                },
                None,
                end,
            )
        }
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
    let kind = raw[1..body_start].trim();
    let kind_lower = kind.to_ascii_lowercase();
    let absolute_body_start = start + body_start + 1;
    let absolute_body_end = end.saturating_sub(1);
    let body = &source[absolute_body_start..absolute_body_end];

    if !is_valid_identifier(kind) {
        if let Some(nested_at) = raw[1..body_start].find('@') {
            let nested_at = start + 1 + nested_at;
            return (
                RawBlock::Other {
                    raw: source[start..nested_at].to_string(),
                    span: start..nested_at,
                },
                None,
                nested_at,
            );
        }
        return (
            RawBlock::Failed {
                raw: raw.to_string(),
                error: format!("entry type {} is invalid", quoted(kind)),
                span: start..end,
            },
            None,
            end,
        );
    }

    match kind_lower.as_str() {
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
            let (key, value) = parse_assignment(body).unwrap_or_default();
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
        }
        _ => match parse_entry(source, start, end, kind, absolute_body_start, body) {
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
    let (key, mut cursor) = parse_entry_key(body)?;
    if !is_valid_entry_key(&key) {
        return Err(format!("entry key {} is invalid", quoted(&key)));
    }

    let mut fields: IndexMap<String, Vec<usize>> = IndexMap::new();
    let mut field_blocks = Vec::new();
    while cursor < body.len() {
        skip_field_gap(body, &mut cursor);
        if cursor >= body.len() {
            break;
        }

        let (name, next_cursor) = parse_field_name(body, cursor)?;
        cursor = next_cursor;
        if name.is_empty() {
            return Err("field name is empty".to_string());
        }

        let value_start;
        let (value, value_end, inner_span, value_mode, value_atoms) = if cursor >= body.len()
            || matches!(body[cursor..].chars().next(), Some(',' | '}' | ')'))
        {
            value_start = cursor;
            (
                String::new(),
                cursor,
                None,
                RawValueMode::Missing,
                Vec::new(),
            )
        } else {
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
            value_start = cursor;
            parse_value(body, cursor, body_start)?
        };
        cursor = value_end;
        let field_id = field_blocks.len();
        fields
            .entry(name.to_ascii_lowercase())
            .or_default()
            .push(field_id);
        field_blocks.push(RawFieldData {
            name: name.clone(),
            value,
            value_mode,
            value_atoms,
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

fn parse_field_name(body: &str, start: usize) -> Result<(String, usize), String> {
    let mut cursor = start;
    let mut name = String::new();
    while cursor < body.len() {
        let ch = body[cursor..].chars().next().unwrap();
        if ch == '=' || ch == ',' || ch == '}' || ch == ')' {
            break;
        }
        if !is_valid_field_name_char(ch) {
            let invalid = ch.to_string();
            return Err(format!(
                "field name contains invalid character {}",
                quoted(&invalid)
            ));
        }
        name.push(ch);
        cursor += ch.len_utf8();
    }
    Ok((name.trim().to_string(), cursor))
}

fn parse_entry_key(body: &str) -> Result<(String, usize), String> {
    let mut cursor = 0;
    skip_field_space(body, &mut cursor);
    if cursor >= body.len() {
        return Ok((String::new(), cursor));
    }

    let mut probe = cursor;
    while probe < body.len() {
        let ch = body[probe..].chars().next().unwrap();
        if ch == ',' {
            return Ok((body[..probe].trim().to_string(), probe + ch.len_utf8()));
        }
        if ch == '=' {
            return Ok((String::new(), cursor));
        }
        if ch == '}' || ch == ')' {
            return Ok((body[..probe].trim().to_string(), probe));
        }
        probe += ch.len_utf8();
    }

    Ok((body.trim().to_string(), body.len()))
}

fn find_at_block_end(source: &str, start: usize) -> Result<usize, (usize, String)> {
    let escape_aware = find_at_block_end_with_escape_mode(source, start, true);
    let permissive = find_at_block_end_with_escape_mode(source, start, false);
    match (escape_aware, permissive) {
        (Ok(escaped_end), Ok(permissive_end))
            if permissive_end < escaped_end && follows_block_boundary(source, permissive_end) =>
        {
            Ok(permissive_end)
        }
        (Ok(escaped_end), _) => Ok(escaped_end),
        (Err((_, error)), Ok(permissive_end))
            if error == "entry ended before closing delimiter" =>
        {
            Ok(permissive_end)
        }
        (Err(error), _) => Err(error),
    }
}

fn follows_block_boundary(source: &str, mut cursor: usize) -> bool {
    while cursor < source.len() {
        let ch = source[cursor..].chars().next().unwrap();
        if !ch.is_whitespace() {
            break;
        }
        cursor += ch.len_utf8();
    }
    cursor >= source.len()
        || source[cursor..].starts_with('%')
        || (source[cursor..].starts_with('@') && is_parseable_at_start(source, cursor))
}

fn find_at_block_end_with_escape_mode(
    source: &str,
    start: usize,
    escape_aware: bool,
) -> Result<usize, (usize, String)> {
    let next_block = find_recovery_block_start(source, start);
    if is_line_style_comment(source, start) {
        return Err((
            take_line(source, start),
            "entry opener is missing".to_string(),
        ));
    }
    let Some(open_rel) = source[start..].find(['{', '(']) else {
        return Err((
            next_block.unwrap_or_else(|| take_line(source, start)),
            "entry opener is missing".to_string(),
        ));
    };
    let open = start + open_rel;
    if let Some(next) = next_block {
        if next < open {
            return Err((next, "entry opener is missing".to_string()));
        }
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
                pos += ch.len_utf8();
                continue;
            } else if ch == '=' {
                in_entry_key = false;
            } else if ch == root_closer {
                return Ok(pos + ch.len_utf8());
            } else {
                pos += ch.len_utf8();
                continue;
            }
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
        } else if escape_aware && ch == '\\' {
            escaped = true;
        } else if ch == '%' && closers.len() == 1 && !raw_comment {
            pos = take_comment(source, pos);
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

fn find_next_root_break(source: &str, start: usize) -> Option<usize> {
    let mut cursor = start;
    while cursor < source.len() {
        let ch = source[cursor..].chars().next().unwrap();
        if ch == '%' {
            return Some(cursor);
        }
        if ch == '@' && is_parseable_at_start(source, cursor) {
            return Some(cursor);
        }
        cursor += ch.len_utf8();
    }
    None
}

fn take_comment(source: &str, start: usize) -> usize {
    let line_end = take_line(source, start);
    let mut cursor = start + 1;
    while cursor < line_end {
        let ch = source[cursor..].chars().next().unwrap();
        if ch == '@' && is_complete_parseable_at_start(source, cursor) {
            return cursor;
        }
        cursor += ch.len_utf8();
    }
    line_end
}

fn is_complete_parseable_at_start(source: &str, start: usize) -> bool {
    if !is_parseable_at_start(source, start) {
        return false;
    }
    match find_at_block_end(source, start) {
        Ok(end) => match find_recovery_block_start(source, start) {
            Some(next) => next >= end,
            None => true,
        },
        Err(_) => false,
    }
}

fn is_parseable_at_start(source: &str, start: usize) -> bool {
    let line_end = take_line(source, start);
    let Some(open_rel) = source[start..line_end].find(['{', '(']) else {
        return is_line_style_comment(source, start);
    };
    let kind = source[start + 1..start + open_rel].trim();
    is_valid_identifier(kind)
}

fn is_line_style_comment(source: &str, start: usize) -> bool {
    let line_start = source[..start].rfind('\n').map_or(0, |index| index + 1);
    if !source[line_start..start].chars().all(char::is_whitespace) {
        return false;
    }
    let line = &source[start..take_line(source, start)];
    let Some(prefix) = line.get(.."@comment".len()) else {
        return false;
    };
    if !prefix.eq_ignore_ascii_case("@comment") {
        return false;
    }
    let rest = line.get("@comment".len()..).unwrap_or_default();
    if !rest
        .chars()
        .next()
        .is_none_or(|ch| ch.is_whitespace() || matches!(ch, '{' | '('))
    {
        return false;
    }
    !matches!(rest.trim_start().chars().next(), Some('{' | '('))
}

pub(super) fn parse_value(
    body: &str,
    start: usize,
    body_offset: usize,
) -> Result<ParsedValue, String> {
    let first = parse_value_atom(body, start, body_offset)?;
    let mut cursor = first.1;
    let mut atoms = first.4.clone();
    let mut expression_cursor = cursor;
    skip_field_space(body, &mut expression_cursor);
    if !body[expression_cursor..].starts_with('#') {
        return Ok(first);
    }

    while expression_cursor < body.len() && body[expression_cursor..].starts_with('#') {
        expression_cursor += 1;
        skip_field_space(body, &mut expression_cursor);
        let (_, atom_end, _, _, atom_atoms) =
            parse_value_atom(body, expression_cursor, body_offset)?;
        atoms.extend(atom_atoms);
        cursor = atom_end;
        expression_cursor = atom_end;
        skip_field_space(body, &mut expression_cursor);
    }

    Ok((
        body[start..cursor].trim().to_string(),
        cursor,
        Some((body_offset + start)..(body_offset + cursor)),
        RawValueMode::Expression,
        atoms,
    ))
}

fn parse_value_atom(body: &str, start: usize, body_offset: usize) -> Result<ParsedValue, String> {
    let Some(ch) = body[start..].chars().next() else {
        return Err("field value is missing".to_string());
    };

    if ch == '{' {
        let end = find_balanced_in_body(body, start, '{', '}')?;
        let inner = (body_offset + start + 1)..(body_offset + end - 1);
        let value = body[start + 1..end - 1].to_string();
        return Ok((
            value.trim().to_string(),
            end,
            Some(inner),
            RawValueMode::Braced,
            vec![RawValueAtom {
                value,
                value_mode: RawValueMode::Braced,
            }],
        ));
    }

    if ch == '"' {
        let end = find_quoted_end(body, start)?;
        let inner = (body_offset + start + 1)..(body_offset + end - 1);
        let value = body[start + 1..end - 1].to_string();
        return Ok((
            value.trim().to_string(),
            end,
            Some(inner),
            RawValueMode::Quoted,
            vec![RawValueAtom {
                value,
                value_mode: RawValueMode::Quoted,
            }],
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
    Ok((
        value.clone(),
        cursor,
        None,
        RawValueMode::Bare,
        vec![RawValueAtom {
            value,
            value_mode: RawValueMode::Bare,
        }],
    ))
}

fn find_balanced_in_body(
    body: &str,
    start: usize,
    opener: char,
    closer: char,
) -> Result<usize, String> {
    let escape_aware = find_balanced_in_body_with_escape_mode(body, start, opener, closer, true);
    let permissive = find_balanced_in_body_with_escape_mode(body, start, opener, closer, false);
    match (escape_aware, permissive) {
        (Ok(escaped_end), Ok(permissive_end))
            if permissive_end < escaped_end
                && follows_non_closing_value_boundary(body, permissive_end) =>
        {
            Ok(permissive_end)
        }
        (Ok(escaped_end), _) => Ok(escaped_end),
        (Err(_), Ok(permissive_end)) => Ok(permissive_end),
        (Err(error), Err(_)) => Err(error),
    }
}

fn follows_non_closing_value_boundary(body: &str, mut cursor: usize) -> bool {
    skip_field_space(body, &mut cursor);
    matches!(body[cursor..].chars().next(), Some(',' | '#'))
}

fn find_balanced_in_body_with_escape_mode(
    body: &str,
    start: usize,
    opener: char,
    closer: char,
    escape_aware: bool,
) -> Result<usize, String> {
    let mut depth = 0usize;
    let mut cursor = start;
    let mut escaped = false;
    while cursor < body.len() {
        let ch = body[cursor..].chars().next().unwrap();
        if escaped {
            escaped = false;
        } else if escape_aware && ch == '\\' {
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
    let (value, end, _, _, _) = parse_value(body, cursor, 0).ok()?;
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
    if let Ok((value, end, _, RawValueMode::Braced | RawValueMode::Quoted, _)) =
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
