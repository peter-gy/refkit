use crate::{
    RawEntryId, RawSyntaxBlock, RawSyntaxDocument, RawSyntaxEntry, RawSyntaxField, RawValueMode,
    normalize_raw_at_command,
};
use std::collections::HashMap;

use super::{TidyOptions, duplicates::DuplicatePlan, options::DEFAULT_FIELD_SORT};

mod sort;
mod value;

pub(crate) fn render_document(
    doc: &RawSyntaxDocument,
    options: &TidyOptions,
    duplicate_plan: &DuplicatePlan,
    key_plan: &HashMap<RawEntryId, String>,
) -> String {
    let mut output = String::new();
    if let Some(sort) = options.sort.as_deref() {
        sort::render_sorted_document(&mut output, doc, options, duplicate_plan, key_plan, sort);
        ensure_final_newline(&mut output);
        return output;
    }

    let context = RenderContext {
        doc,
        options,
        duplicate_plan,
        key_plan,
    };
    let mut state = LinearRenderState::default();

    for block in &doc.blocks {
        render_block(&mut output, block, &context, &mut state);
    }

    ensure_final_newline(&mut output);
    output
}

struct RenderContext<'a> {
    doc: &'a RawSyntaxDocument,
    options: &'a TidyOptions,
    duplicate_plan: &'a DuplicatePlan,
    key_plan: &'a HashMap<RawEntryId, String>,
}

#[derive(Default)]
struct LinearRenderState {
    wrote_block: bool,
    previous_was_comment: bool,
    previous_comment_ended_line: bool,
    previous_comment_was_text: bool,
    pending_whitespace: String,
    suppress_next_whitespace: bool,
}

fn render_block(
    output: &mut String,
    block: &RawSyntaxBlock,
    context: &RenderContext<'_>,
    state: &mut LinearRenderState,
) {
    match block {
        RawSyntaxBlock::Entry { id, .. } => {
            if context.duplicate_plan.should_skip(*id) {
                state.pending_whitespace.clear();
                state.suppress_next_whitespace = true;
                return;
            }
            if let Some(entry) = context.doc.entries.iter().find(|entry| entry.id == *id) {
                state.suppress_next_whitespace = false;
                state.pending_whitespace.clear();
                separate_block(
                    output,
                    state.wrote_block,
                    state.previous_was_comment,
                    context.options,
                );
                render_entry(
                    output,
                    context.duplicate_plan.entry(entry),
                    context.options,
                    context.key_plan,
                );
                state.wrote_block = true;
                state.previous_was_comment = false;
                state.previous_comment_ended_line = false;
                state.previous_comment_was_text = false;
            }
        }
        RawSyntaxBlock::Comment { raw, .. } => {
            if !context.options.strip_comments {
                state.suppress_next_whitespace = false;
                render_comment_block(output, raw, context.options, state);
            }
        }
        RawSyntaxBlock::Preamble { raw, .. } | RawSyntaxBlock::StringDef { raw, .. } => {
            state.suppress_next_whitespace = false;
            state.pending_whitespace.clear();
            separate_block(
                output,
                state.wrote_block,
                state.previous_was_comment,
                context.options,
            );
            output.push_str(&normalize_raw_at_command(raw.trim()));
            output.push('\n');
            state.wrote_block = true;
            state.previous_was_comment = false;
            state.previous_comment_ended_line = false;
            state.previous_comment_was_text = false;
        }
        RawSyntaxBlock::Whitespace { raw, .. } => {
            if !state.suppress_next_whitespace {
                state.pending_whitespace.push_str(raw);
            }
        }
        RawSyntaxBlock::Failed { .. } => {}
        RawSyntaxBlock::Other { raw, .. } => {
            if !context.options.strip_comments {
                state.suppress_next_whitespace = false;
                render_comment_block(output, raw, context.options, state);
            }
        }
    }
}

fn render_comment_block(
    output: &mut String,
    raw: &str,
    options: &TidyOptions,
    state: &mut LinearRenderState,
) {
    if options.tidy_comments {
        let prefix = tidy_comment_prefix(
            &state.pending_whitespace,
            state.previous_was_comment,
            state.previous_comment_ended_line,
            state.previous_comment_was_text,
        );
        state.pending_whitespace.clear();
        separate_block(
            output,
            state.wrote_block,
            state.previous_was_comment,
            options,
        );
        output.push_str(&prefix);
        output.push_str(&normalize_raw_at_command(raw.trim()));
        output.push('\n');
    } else {
        if !state.pending_whitespace.is_empty() {
            push_preserved_comment_whitespace(output, &state.pending_whitespace);
            state.pending_whitespace.clear();
        } else {
            separate_block(
                output,
                state.wrote_block,
                state.previous_was_comment,
                options,
            );
        }
        output.push_str(&normalize_raw_at_command(raw));
        if !output.ends_with('\n') {
            output.push('\n');
        }
    }
    state.wrote_block = true;
    state.previous_was_comment = true;
    state.previous_comment_ended_line = raw.ends_with('\n');
    state.previous_comment_was_text = !raw.trim_start().starts_with('@');
}

fn push_preserved_comment_whitespace(output: &mut String, pending_whitespace: &str) {
    if output.ends_with('\n') {
        if let Some(rest) = pending_whitespace.strip_prefix('\n') {
            output.push_str(rest);
        } else {
            output.push_str(pending_whitespace);
        }
    } else {
        output.push_str(pending_whitespace);
    }
}

fn tidy_comment_prefix(
    pending_whitespace: &str,
    previous_was_comment: bool,
    previous_comment_ended_line: bool,
    previous_comment_was_text: bool,
) -> String {
    if !previous_was_comment || !previous_comment_was_text {
        return String::new();
    }
    if pending_whitespace.contains('\n') {
        return "\n".to_string();
    }
    if previous_comment_ended_line
        && pending_whitespace
            .chars()
            .all(|ch| matches!(ch, ' ' | '\t'))
    {
        return pending_whitespace.to_string();
    }
    String::new()
}

fn ensure_final_newline(output: &mut String) {
    if !output.ends_with('\n') {
        output.push('\n');
    }
}

fn separate_block(
    output: &mut String,
    wrote_block: bool,
    previous_was_comment: bool,
    options: &TidyOptions,
) {
    if !wrote_block {
        return;
    }
    if !output.ends_with('\n') {
        output.push('\n');
    }
    if options.blank_lines && !previous_was_comment && !output.ends_with("\n\n") {
        output.push('\n');
    }
}

fn render_entry(
    output: &mut String,
    entry: &RawSyntaxEntry,
    options: &TidyOptions,
    key_plan: &HashMap<RawEntryId, String>,
) {
    let entry_kind = if options.lowercase {
        entry.kind.to_ascii_lowercase()
    } else {
        entry.kind.clone()
    };
    let indent = if options.tab {
        "\t".to_string()
    } else {
        " ".repeat(options.space)
    };

    output.push('@');
    output.push_str(&entry_kind);
    output.push('{');
    let key = key_plan.get(&entry.id).unwrap_or(&entry.key);
    output.push_str(key);
    let fields = renderable_fields(entry, options);
    if !fields.is_empty() {
        if !key.is_empty() {
            output.push(',');
        }
        output.push('\n');
    } else {
        output.push(',');
        output.push('\n');
    }

    for (index, field) in fields.iter().enumerate() {
        let field_name = render_field_name(field, options);
        output.push_str(&indent);
        output.push_str(&field_name);
        if field.value_mode != RawValueMode::Missing {
            output.push_str(&separator(&field_name, options));
            output.push_str(&render_value(field, options));
        }
        if index + 1 < fields.len() || options.trailing_commas {
            output.push(',');
        }
        output.push('\n');
    }

    output.push('}');
    output.push('\n');
}

fn renderable_fields<'a>(
    entry: &'a RawSyntaxEntry,
    options: &TidyOptions,
) -> Vec<&'a RawSyntaxField> {
    let omit = options
        .omit
        .iter()
        .map(|field| field.to_ascii_lowercase())
        .collect::<std::collections::BTreeSet<_>>();
    let mut seen = std::collections::BTreeSet::new();
    let mut fields = Vec::new();

    for field in &entry.fields {
        let field_key = field.name.to_ascii_lowercase();
        if omit.contains(&field_key) {
            continue;
        }
        if options.remove_duplicate_fields && !seen.insert(field_key) {
            continue;
        }
        if options.remove_empty_fields && field.value.trim().is_empty() {
            continue;
        }
        fields.push(field);
    }

    sort_fields(fields, options)
}

fn sort_fields<'a>(
    mut fields: Vec<&'a RawSyntaxField>,
    options: &TidyOptions,
) -> Vec<&'a RawSyntaxField> {
    let Some(order) = options.sort_fields.as_ref() else {
        return fields;
    };
    let order = if order.is_empty() {
        DEFAULT_FIELD_SORT
            .iter()
            .map(|field| (*field).to_string())
            .collect::<Vec<_>>()
    } else {
        order.clone()
    };

    fields.sort_by(|left, right| {
        let left_order = field_order(&left.name, &order);
        let right_order = field_order(&right.name, &order);
        match (left_order, right_order) {
            (Some(left_order), Some(right_order)) => left_order.cmp(&right_order),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
    fields
}

fn field_order(field: &str, order: &[String]) -> Option<usize> {
    let field = field.to_ascii_lowercase();
    order
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(&field))
}

fn render_field_name(field: &RawSyntaxField, options: &TidyOptions) -> String {
    if options.lowercase {
        field.name.to_ascii_lowercase()
    } else {
        field.name.clone()
    }
}

fn separator(field_name: &str, options: &TidyOptions) -> String {
    let Some(align) = options.align else {
        return " = ".to_string();
    };
    let width = align.saturating_sub(field_name.len()).max(1);
    format!("{}= ", " ".repeat(width))
}

fn render_value(field: &RawSyntaxField, options: &TidyOptions) -> String {
    value::render_value(field, options)
}
