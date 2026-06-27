use std::ops::Range;

use super::{
    RawBlock, RawDocumentData, RawEditError, RawEntryData, RawFieldData, RawValueMode,
    is_safe_bare_value,
};
use crate::quoted;

pub fn set_raw_field_value(
    doc: &mut RawDocumentData,
    entry_id: usize,
    field_id: usize,
    value: String,
) -> Result<(), RawEditError> {
    let field = doc
        .entry_blocks
        .get_mut(entry_id)
        .and_then(|entry| entry.field_blocks.get_mut(field_id))
        .ok_or(RawEditError::MissingField { entry_id, field_id })?;

    validate_field_value(&value, field.value_mode).map_err(RawEditError::InvalidValue)?;
    field.value = value;
    field.changed = true;
    Ok(())
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
                if entry.field_blocks.iter().any(|field| field.changed) {
                    output.push_str(&patch_entry(entry)?);
                } else {
                    output.push_str(&entry.raw);
                }
            }
        }
    }
    Ok(output)
}

fn patch_entry(entry: &RawEntryData) -> Result<String, String> {
    let mut fields = entry
        .field_blocks
        .iter()
        .filter(|field| field.changed)
        .collect::<Vec<_>>();
    fields.sort_by_key(|field| patch_field_value(field).0.start);

    let mut output = String::with_capacity(entry.raw.len());
    let mut cursor = entry.span.start;
    for field in fields {
        let (span, value) = patch_field_value(field);
        if span.start < entry.span.start || span.end > entry.span.end || span.start < cursor {
            return Err(format!(
                "invalid source span for BibTeX field {}",
                field.name
            ));
        }

        output.push_str(&entry.raw[cursor - entry.span.start..span.start - entry.span.start]);
        output.push_str(&value);
        cursor = span.end;
    }
    output.push_str(&entry.raw[cursor - entry.span.start..]);
    Ok(output)
}

fn patch_field_value(field: &RawFieldData) -> (Range<usize>, String) {
    if field.value_mode == RawValueMode::Quoted && contains_unescaped(&field.value, '"') {
        return (field.patch_span.clone(), format!("{{{}}}", field.value));
    }
    (field.span.clone(), render_field_value(field))
}

fn render_field_value(field: &RawFieldData) -> String {
    match field.value_mode {
        RawValueMode::Bare if !is_safe_bare_value(&field.value) => {
            format!("{{{}}}", field.value)
        }
        RawValueMode::Expression => format!("{{{}}}", field.value),
        RawValueMode::Bare | RawValueMode::Braced | RawValueMode::Quoted => field.value.clone(),
    }
}

fn validate_field_value(value: &str, value_mode: RawValueMode) -> Result<(), String> {
    match value_mode {
        RawValueMode::Bare if is_safe_bare_value(value) => Ok(()),
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
