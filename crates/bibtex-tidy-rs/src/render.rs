use refkit_core::{
    RawEntryId, RawSyntaxBlock, RawSyntaxDocument, RawSyntaxEntry, RawSyntaxField, RawValueAtom,
    RawValueMode, normalize_raw_at_command,
};
use std::collections::{BTreeMap, HashMap};

use crate::{
    TidyOptions,
    duplicates::DuplicatePlan,
    latex::{double_enclose, flatten_unprotected_braces, strip_simple_outer_brace},
    options::DEFAULT_FIELD_SORT,
    unicode::latex_escape,
};

const MISSING_SORT_VALUE: &str = "\u{fff0}";

pub(crate) fn render_document(
    doc: &RawSyntaxDocument,
    options: &TidyOptions,
    duplicate_plan: &DuplicatePlan,
    key_plan: &HashMap<RawEntryId, String>,
) -> String {
    let mut output = String::new();
    if let Some(sort) = options.sort.as_deref() {
        render_sorted_document(&mut output, doc, options, duplicate_plan, key_plan, sort);
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

#[derive(Clone)]
struct SortedBlock<'a> {
    block: &'a RawSyntaxBlock,
    sort_index: BTreeMap<String, String>,
}

fn render_sorted_document(
    output: &mut String,
    doc: &RawSyntaxDocument,
    options: &TidyOptions,
    duplicate_plan: &DuplicatePlan,
    key_plan: &HashMap<RawEntryId, String>,
    sort: &[String],
) {
    let sort = if sort.is_empty() {
        vec!["key".to_string()]
    } else {
        sort.to_vec()
    };
    let mut blocks = sorted_blocks(doc, duplicate_plan, &sort);
    for prefixed_key in sort.iter().rev() {
        let descending = prefixed_key.starts_with('-');
        let key = prefixed_key.strip_prefix('-').unwrap_or(prefixed_key);
        blocks.sort_by(|left, right| {
            match (
                present_sort_value(&left.sort_index, key),
                present_sort_value(&right.sort_index, key),
            ) {
                (Some(left_value), Some(right_value)) if descending => right_value.cmp(left_value),
                (Some(left_value), Some(right_value)) => left_value.cmp(right_value),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
    }

    let context = RenderContext {
        doc,
        options,
        duplicate_plan,
        key_plan,
    };
    let mut state = LinearRenderState::default();
    for block in blocks {
        render_block(output, block.block, &context, &mut state);
    }
}

fn sorted_blocks<'a>(
    doc: &'a RawSyntaxDocument,
    duplicate_plan: &'a DuplicatePlan,
    sort: &[String],
) -> Vec<SortedBlock<'a>> {
    let includes_special = sort
        .iter()
        .any(|key| key.strip_prefix('-').unwrap_or(key) == "special");
    let mut blocks = Vec::new();
    let mut preceding_meta = Vec::new();

    for block in &doc.blocks {
        if matches!(
            block,
            RawSyntaxBlock::Whitespace { .. } | RawSyntaxBlock::Failed { .. }
        ) {
            continue;
        }

        let entry = match block {
            RawSyntaxBlock::Entry { id, .. } => {
                if duplicate_plan.should_skip(*id) {
                    continue;
                }
                doc.entries
                    .iter()
                    .find(|entry| entry.id == *id)
                    .map(|entry| duplicate_plan.entry(entry))
            }
            _ => None,
        };

        if matches!(
            block,
            RawSyntaxBlock::Comment { .. } | RawSyntaxBlock::Other { .. }
        ) || (entry.is_none() && !includes_special)
        {
            let index = blocks.len();
            blocks.push(SortedBlock {
                block,
                sort_index: BTreeMap::new(),
            });
            preceding_meta.push(index);
            continue;
        }

        let sort_index = block_sort_index(block, entry, sort);
        let index = blocks.len();
        blocks.push(SortedBlock {
            block,
            sort_index: sort_index.clone(),
        });
        for meta_index in preceding_meta.drain(..) {
            blocks[meta_index].sort_index = sort_index.clone();
        }
        if entry.is_none() && blocks[index].sort_index.is_empty() {
            preceding_meta.push(index);
        }
    }

    blocks
}

fn present_sort_value<'a>(sort_index: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    match sort_index.get(key).map(String::as_str) {
        Some(MISSING_SORT_VALUE) | None => None,
        Some(value) => Some(value),
    }
}

fn block_sort_index(
    block: &RawSyntaxBlock,
    entry: Option<&RawSyntaxEntry>,
    sort: &[String],
) -> BTreeMap<String, String> {
    let mut sort_index = BTreeMap::new();
    for prefixed_key in sort {
        let key = prefixed_key.strip_prefix('-').unwrap_or(prefixed_key);
        let Some(value) = block_sort_value(block, entry, key) else {
            continue;
        };
        sort_index.insert(key.to_string(), value);
    }
    sort_index
}

fn block_sort_value(
    block: &RawSyntaxBlock,
    entry: Option<&RawSyntaxEntry>,
    key: &str,
) -> Option<String> {
    match key {
        "special" => Some(if is_special_block(block, entry) {
            "0".to_string()
        } else {
            "1".to_string()
        }),
        "type" => Some(block_command(block, entry)?.to_ascii_lowercase()),
        "key" => entry.map(|entry| entry.key.to_ascii_lowercase()),
        "month" => entry.map(|entry| sort_value(entry, key)),
        field => entry.map(|entry| sort_value(entry, field)),
    }
}

fn block_command<'a>(
    block: &'a RawSyntaxBlock,
    entry: Option<&'a RawSyntaxEntry>,
) -> Option<&'a str> {
    match block {
        RawSyntaxBlock::Preamble { .. } => Some("preamble"),
        RawSyntaxBlock::StringDef { .. } => Some("string"),
        RawSyntaxBlock::Entry { .. } => entry.map(|entry| entry.kind.as_str()),
        _ => None,
    }
}

fn is_special_block(block: &RawSyntaxBlock, entry: Option<&RawSyntaxEntry>) -> bool {
    match block_command(block, entry).map(str::to_ascii_lowercase) {
        Some(command) => matches!(command.as_str(), "string" | "preamble" | "set" | "xdata"),
        None => false,
    }
}

fn sort_value(entry: &RawSyntaxEntry, key: &str) -> String {
    match key {
        "key" => entry.key.to_ascii_lowercase(),
        "type" => entry.kind.to_ascii_lowercase(),
        "month" => entry
            .fields
            .iter()
            .find(|field| field.name.eq_ignore_ascii_case("month"))
            .map(|field| month_sort_value(&field.value))
            .unwrap_or_else(missing_sort_value),
        field_name => entry
            .fields
            .iter()
            .find(|field| field.name.eq_ignore_ascii_case(field_name))
            .map(|field| scalar_sort_value(&field.value))
            .unwrap_or_else(missing_sort_value),
    }
}

fn month_sort_value(value: &str) -> String {
    let month = value.trim().to_ascii_lowercase();
    let month_index = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ]
    .iter()
    .position(|candidate| *candidate == month);
    month_index
        .map(|index| format!("0:{index:02}"))
        .unwrap_or_else(|| scalar_sort_value(value))
}

fn scalar_sort_value(value: &str) -> String {
    let value = value.trim().to_ascii_lowercase();
    if value.chars().all(|ch| ch.is_ascii_digit()) && !value.is_empty() {
        format!("0:{value:0>50}")
    } else {
        format!("1:{value}")
    }
}

fn missing_sort_value() -> String {
    MISSING_SORT_VALUE.to_string()
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
    if field.value_mode == RawValueMode::Expression {
        return render_expression_atoms(&field.value_atoms);
    }

    let (value, original_mode) = transformed_value(field, options);

    let value_mode = if options.months && field.name.eq_ignore_ascii_case("month") {
        original_mode
    } else if options.numeric && is_numeric_value(&value) {
        RawValueMode::Bare
    } else if options.curly && !is_bare_month(field, &value) {
        RawValueMode::Braced
    } else {
        original_mode
    };
    let value = if value_mode == RawValueMode::Braced {
        wrap_braced_value(field, &value, options)
    } else {
        value
    };

    match value_mode {
        RawValueMode::Bare | RawValueMode::Missing => value,
        RawValueMode::Braced | RawValueMode::Expression => format!("{{{value}}}"),
        RawValueMode::Quoted => format!("\"{value}\""),
    }
}

fn render_expression_atoms(atoms: &[RawValueAtom]) -> String {
    atoms
        .iter()
        .map(render_expression_atom)
        .filter(|atom| !atom.is_empty())
        .collect::<Vec<_>>()
        .join(" # ")
}

fn render_expression_atom(atom: &RawValueAtom) -> String {
    match atom.value_mode {
        RawValueMode::Bare | RawValueMode::Missing | RawValueMode::Expression => {
            atom.value.trim().to_string()
        }
        RawValueMode::Braced => format!("{{{}}}", atom.value),
        RawValueMode::Quoted => format!("\"{}\"", atom.value),
    }
}

fn transformed_value(field: &RawSyntaxField, options: &TidyOptions) -> (String, RawValueMode) {
    if options.months && field.name.eq_ignore_ascii_case("month") {
        if let Some(month) = abbreviate_month(&field.value) {
            return (month.to_string(), RawValueMode::Bare);
        }
    }

    let mut value = field.value.clone();

    if options.drop_all_caps {
        value = drop_all_caps(&value);
    }

    if options.encode_urls && field.name.eq_ignore_ascii_case("url") {
        value = encode_url(&value);
    }

    if options.escape && !is_verbatim_field(&field.name) {
        value = escape_characters(&value);
    }

    if field.name.eq_ignore_ascii_case("pages") {
        value = format_page_range(&value);
    }

    if field.name.eq_ignore_ascii_case("author") {
        if let Some(max_authors) = options.max_authors {
            value = limit_authors(&value, max_authors);
        }
    }

    if options.strip_enclosing_braces {
        value = strip_simple_outer_brace(&value);
    }

    if should_transform_field(&field.name, options.remove_braces.as_deref()) {
        value = flatten_unprotected_braces(&value);
    }

    if should_transform_field(&field.name, options.enclosing_braces.as_deref()) {
        value = double_enclose(&value);
    }

    if field.value_mode == RawValueMode::Braced {
        value = unwrap_text(&value).trim().to_string();
    }

    (value, field.value_mode)
}

fn should_transform_field(field: &str, fields: Option<&[String]>) -> bool {
    fields.is_some_and(|fields| {
        fields
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(field))
    })
}

fn abbreviate_month(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "jan" | "january" => Some("jan"),
        "2" | "feb" | "february" => Some("feb"),
        "3" | "mar" | "march" => Some("mar"),
        "4" | "apr" | "april" => Some("apr"),
        "5" | "may" => Some("may"),
        "6" | "jun" | "june" => Some("jun"),
        "7" | "jul" | "july" => Some("jul"),
        "8" | "aug" | "august" => Some("aug"),
        "9" | "sep" | "september" => Some("sep"),
        "10" | "oct" | "october" => Some("oct"),
        "11" | "nov" | "november" => Some("nov"),
        "12" | "dec" | "december" => Some("dec"),
        _ => None,
    }
}

fn encode_url(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' && chars.peek() == Some(&'_') {
            chars.next();
            output.push_str("\\%5F");
        } else if ch == '_' {
            output.push_str("\\%5F");
        } else {
            output.push(ch);
        }
    }
    output
}

fn unwrap_text(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut pending_space = false;
    let mut newline_count = 0usize;
    for ch in value.chars() {
        if ch == '\n' {
            newline_count += 1;
            pending_space = false;
            continue;
        }
        if ch.is_whitespace() {
            if newline_count >= 2 {
                continue;
            }
            pending_space = true;
            continue;
        }
        if newline_count >= 2 {
            while output.ends_with(' ') {
                output.pop();
            }
            output.push_str("\n\n");
        } else if (newline_count == 1 || pending_space) && !output.is_empty() {
            output.push(' ');
        }
        newline_count = 0;
        pending_space = false;
        output.push(ch);
    }
    output
}

fn wrap_braced_value(field: &RawSyntaxField, value: &str, options: &TidyOptions) -> String {
    let indent = if options.tab {
        "\t".to_string()
    } else {
        " ".repeat(options.space)
    };
    let field_name = render_field_name(field, options);
    let inline_len =
        indent.len() + field_name.len() + separator(&field_name, options).len() + value.len() + 2;
    let multi_paragraph = value.contains("\n\n");
    let should_wrap = options.wrap.is_some_and(|wrap| inline_len > wrap) || multi_paragraph;
    if !should_wrap {
        return value.to_string();
    }

    let value_indent = indent.repeat(2);
    let paragraphs = value
        .split("\n\n")
        .map(|paragraph| {
            if let Some(wrap) = options.wrap {
                wrap_text(paragraph, wrap.saturating_sub(value_indent.len()))
                    .join(&format!("\n{value_indent}"))
            } else {
                paragraph.to_string()
            }
        })
        .collect::<Vec<_>>();
    format!(
        "\n{value_indent}{}\n{indent}",
        paragraphs.join(&format!("\n\n{value_indent}"))
    )
}

fn wrap_text(line: &str, line_width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for (index, word) in line.split(' ').enumerate() {
        if current.len() + word.len() + 1 > line_width && index > 0 {
            lines.push(current.trim().to_string());
            current.clear();
        }
        current.push_str(word);
        current.push(' ');
    }
    lines.push(current.trim().to_string());
    lines
}

fn escape_characters(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len());
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] == '$' {
            if let Some(end) = chars[index + 1..].iter().position(|ch| *ch == '$') {
                if end > 0 {
                    let end = index + end + 1;
                    output.extend(chars[index..=end].iter());
                    index = end + 1;
                    continue;
                }
            }
        }

        if chars[index] == '\\' {
            output.push(chars[index]);
            index += 1;
            if let Some(next) = chars.get(index) {
                output.push(*next);
                index += 1;
            }
            continue;
        }

        if let Some(escaped) = latex_escape(chars[index]) {
            output.push_str(escaped);
        } else {
            output.push(chars[index]);
        }
        index += 1;
    }
    output
}

fn is_verbatim_field(field: &str) -> bool {
    matches!(
        field.to_ascii_lowercase().as_str(),
        "url" | "doi" | "eprint" | "file" | "verba" | "verbb" | "verbc" | "pdf"
    )
}

fn drop_all_caps(value: &str) -> String {
    if value.chars().any(|ch| ch.is_lowercase()) {
        return value.to_string();
    }

    let mut output = String::with_capacity(value.len());
    let mut word = String::new();
    for ch in value.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            word.push(ch);
        } else {
            push_title_word(&mut output, &word);
            word.clear();
            output.push(ch);
        }
    }
    push_title_word(&mut output, &word);
    output
}

fn push_title_word(output: &mut String, word: &str) {
    if word.is_empty() {
        return;
    }
    if is_roman_numeral(word) {
        output.push_str(word);
        return;
    }

    let mut chars = word.chars();
    if let Some(first) = chars.next() {
        output.extend(first.to_uppercase());
        for ch in chars {
            output.extend(ch.to_lowercase());
        }
    }
}

fn is_roman_numeral(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let chars = word.chars().collect::<Vec<_>>();
    let mut index = 0usize;
    roman_take(&chars, &mut index, 'M', 0, 4);
    roman_take_digit(&chars, &mut index, 'C', 'D', 'M');
    roman_take_digit(&chars, &mut index, 'X', 'L', 'C');
    roman_take_digit(&chars, &mut index, 'I', 'V', 'X');
    index == chars.len()
}

fn roman_take(chars: &[char], index: &mut usize, ch: char, min: usize, max: usize) -> usize {
    let start = *index;
    while *index < chars.len() && chars[*index] == ch && *index - start < max {
        *index += 1;
    }
    let count = *index - start;
    if count < min {
        *index = start;
        0
    } else {
        count
    }
}

fn roman_take_pair(chars: &[char], index: &mut usize, first: char, second: char) -> bool {
    if chars.get(*index) == Some(&first) && chars.get(*index + 1) == Some(&second) {
        *index += 2;
        true
    } else {
        false
    }
}

fn roman_take_digit(chars: &[char], index: &mut usize, one: char, five: char, ten: char) {
    if roman_take_pair(chars, index, one, ten) || roman_take_pair(chars, index, one, five) {
        return;
    }
    roman_take(chars, index, five, 0, 1);
    roman_take(chars, index, one, 0, 3);
}

fn limit_authors(value: &str, max_authors: usize) -> String {
    let authors = value.split(" and ").collect::<Vec<_>>();
    if authors.len() > max_authors {
        authors
            .into_iter()
            .take(max_authors)
            .chain(std::iter::once("others"))
            .collect::<Vec<_>>()
            .join(" and ")
    } else {
        value.to_string()
    }
}

fn is_numeric_value(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('1'..='9')) && chars.all(|ch| ch.is_ascii_digit())
}

fn is_bare_month(field: &RawSyntaxField, value: &str) -> bool {
    field.name.eq_ignore_ascii_case("month")
        && matches!(
            value.to_ascii_lowercase().as_str(),
            "jan"
                | "feb"
                | "mar"
                | "apr"
                | "may"
                | "jun"
                | "jul"
                | "aug"
                | "sep"
                | "oct"
                | "nov"
                | "dec"
        )
        && field.value_mode == RawValueMode::Bare
}

fn format_page_range(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len() + 1);
    for index in 0..chars.len() {
        let ch = chars[index];
        if ch == '-'
            && chars
                .get(index.wrapping_sub(1))
                .is_some_and(|left| left.is_ascii_digit())
            && chars
                .get(index + 1)
                .is_some_and(|right| right.is_ascii_digit())
            && chars.get(index + 1) != Some(&'-')
            && chars.get(index.wrapping_sub(1)) != Some(&'-')
        {
            output.push_str("--");
        } else {
            output.push(ch);
        }
    }
    output
}
