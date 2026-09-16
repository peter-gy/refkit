use std::ops::Range;

use super::{RawBlock, RawDocumentData, RawFieldData, RawValueMode, is_safe_bare_value};
use crate::quoted;

pub(super) fn prepare_field_edit(
    field: &RawFieldData,
    value: &str,
) -> Result<(Range<usize>, String), String> {
    validate_field_value(value, field.value_mode)?;
    Ok(patch_field_value(field, value))
}

pub(super) fn prepare_new_value(value: &str) -> Result<String, String> {
    validate_braced_field_value(value)?;
    Ok(format!("{{{value}}}"))
}

pub(super) fn prepare_expression(value: &str) -> Result<(String, String), String> {
    let text = value.trim();
    if text.is_empty() {
        return Err("BibTeX value expression must not be empty".into());
    }
    crate::library::validate_literal(text, 0).map_err(|error| error.message)?;
    let parsed = super::parse::parse_value(text, 0, 0)?;
    if parsed.1 != text.len() || parsed.3 == RawValueMode::Missing {
        return Err("Expected one complete BibTeX value expression".into());
    }
    Ok((text.into(), parsed.0))
}

pub fn render_raw_document(data: &RawDocumentData) -> Result<String, String> {
    let mut output = String::with_capacity(
        data.blocks
            .last()
            .map(|block| block.span().end)
            .unwrap_or_default(),
    );
    for block in &data.blocks {
        match block {
            RawBlock::Whitespace { raw, .. }
            | RawBlock::Comment { raw, .. }
            | RawBlock::Preamble { raw, .. }
            | RawBlock::StringDef { raw, .. }
            | RawBlock::Failed { raw, .. }
            | RawBlock::Other { raw, .. } => output.push_str(raw),
            RawBlock::Entry { id, key, .. } => {
                let entry = data
                    .entry_blocks
                    .get(*id)
                    .ok_or_else(|| format!("missing BibTeX entry {}", quoted(key)))?;
                output.push_str(&entry.raw);
            }
        }
    }
    Ok(output)
}

fn patch_field_value(field: &RawFieldData, value: &str) -> (Range<usize>, String) {
    if field.value_mode == RawValueMode::Quoted && contains_unescaped(value, '"') {
        return (field.patch_span.clone(), format!("{{{value}}}"));
    }
    (
        field.span.clone(),
        render_field_value(field.value_mode, value),
    )
}

fn render_field_value(mode: RawValueMode, value: &str) -> String {
    match mode {
        RawValueMode::Bare if !is_safe_bare_value(value) => {
            format!("{{{value}}}")
        }
        RawValueMode::Missing => String::new(),
        RawValueMode::Expression => format!("{{{value}}}"),
        RawValueMode::Bare | RawValueMode::Braced | RawValueMode::Quoted => value.to_string(),
    }
}

fn validate_field_value(value: &str, value_mode: RawValueMode) -> Result<(), String> {
    match value_mode {
        RawValueMode::Bare if is_safe_bare_value(value) => Ok(()),
        RawValueMode::Missing if value.is_empty() => Ok(()),
        RawValueMode::Missing => {
            Err("BibTeX field without an assignment cannot be edited to a value".to_string())
        }
        RawValueMode::Bare | RawValueMode::Braced | RawValueMode::Expression => {
            validate_braced_field_value(value)
        }
        RawValueMode::Quoted => validate_quoted_field_value(value),
    }
}

fn validate_braced_field_value(value: &str) -> Result<(), String> {
    if value.contains('\n')
        || contains_unescaped(value, '%')
        || ends_with_unescaped_backslash(value)
    {
        return Err("BibTeX field value contains an unsafe braced delimiter".to_string());
    }
    if !has_balanced_unescaped_braces(value) {
        return Err("BibTeX field value contains an unsafe braced delimiter".to_string());
    }
    Ok(())
}

fn validate_quoted_field_value(value: &str) -> Result<(), String> {
    if value.contains('\n')
        || contains_unprotected_unescaped_quote(value)
        || contains_unescaped(value, '%')
        || ends_with_unescaped_backslash(value)
    {
        return Err("BibTeX field value contains an unsafe quoted delimiter".to_string());
    }
    if !has_balanced_unescaped_braces(value) {
        return Err("BibTeX field value contains an unsafe quoted delimiter".to_string());
    }
    Ok(())
}

fn contains_unprotected_unescaped_quote(value: &str) -> bool {
    let mut depth = 0usize;
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' && depth > 0 {
            depth -= 1;
        } else if ch == '"' && depth == 0 {
            return true;
        }
    }
    false
}

fn has_balanced_unescaped_braces(value: &str) -> bool {
    let mut depth = 0usize;
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            let Some(next_depth) = depth.checked_sub(1) else {
                return false;
            };
            depth = next_depth;
        }
    }
    depth == 0
}

fn contains_unescaped(value: &str, target: char) -> bool {
    let mut escaped = false;
    for ch in value.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == target {
            return true;
        }
    }
    false
}

fn ends_with_unescaped_backslash(value: &str) -> bool {
    let mut count = 0usize;
    for ch in value.chars().rev() {
        if ch == '\\' {
            count += 1;
        } else {
            break;
        }
    }
    count % 2 == 1
}
