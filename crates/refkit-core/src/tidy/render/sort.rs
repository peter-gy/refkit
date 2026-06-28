use std::collections::{BTreeMap, HashMap};

use crate::{RawEntryId, RawSyntaxBlock, RawSyntaxDocument, RawSyntaxEntry};

use super::{LinearRenderState, RenderContext, render_block};
use crate::tidy::{TidyOptions, duplicates::DuplicatePlan};

const MISSING_SORT_VALUE: &str = "\u{fff0}";

#[derive(Clone)]
struct SortedBlock<'a> {
    block: &'a RawSyntaxBlock,
    sort_index: BTreeMap<String, String>,
}

pub(super) fn render_sorted_document(
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
