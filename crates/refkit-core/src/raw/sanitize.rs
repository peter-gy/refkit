use std::collections::BTreeSet;
use std::ops::Range;

use biblatex::RawBibliography;

use super::{RawBlock, RawEntryData, parse_raw_document};
use crate::quoted;

pub fn sanitize_biblatex_for_library(
    source: &str,
    validate_entries: bool,
    collect_diagnostics: bool,
) -> (String, Vec<String>) {
    let data = parse_raw_document(source);
    let mut output = String::with_capacity(source.len());
    let mut diagnostics = Vec::new();
    let mut seen_entries = BTreeSet::new();
    for block in &data.blocks {
        match block {
            RawBlock::Whitespace { raw, .. }
            | RawBlock::Comment { raw, .. }
            | RawBlock::Preamble { raw, .. }
            | RawBlock::StringDef { raw, .. } => output.push_str(raw),
            RawBlock::Entry { id, .. } => {
                if let Some(entry) = data.entry_blocks.get(*id) {
                    if !seen_entries.insert(entry.key.clone()) {
                        push_diagnostic(
                            &mut diagnostics,
                            collect_diagnostics,
                            format!(
                                "ignored duplicate BibTeX entry key {} at {}..{}",
                                quoted(&entry.key),
                                entry.span.start,
                                entry.span.end
                            ),
                        );
                    } else if validate_entries && let Err(err) = RawBibliography::parse(&entry.raw)
                    {
                        push_diagnostic(
                            &mut diagnostics,
                            collect_diagnostics,
                            format!(
                                "ignored BibTeX entry {} at {}..{} because syntax validation failed: {}",
                                quoted(&entry.key),
                                entry.span.start,
                                entry.span.end,
                                err
                            ),
                        );
                    } else {
                        output.push_str(&entry.raw);
                    }
                }
            }
            RawBlock::Failed { error, span, .. } => {
                push_diagnostic(
                    &mut diagnostics,
                    collect_diagnostics,
                    format!(
                        "ignored malformed BibTeX block at {}..{}: {}",
                        span.start, span.end, error
                    ),
                );
            }
            RawBlock::Other { raw, span } => {
                if !raw.trim().is_empty() {
                    push_diagnostic(
                        &mut diagnostics,
                        collect_diagnostics,
                        format!("ignored raw BibTeX text at {}..{}", span.start, span.end),
                    );
                }
            }
        }
    }
    (output, diagnostics)
}

fn push_diagnostic(diagnostics: &mut Vec<String>, collect: bool, message: String) {
    if collect {
        diagnostics.push(message);
    }
}

pub fn sanitize_biblatex_for_library_literals(
    source: &str,
    collect_diagnostics: bool,
) -> (String, Vec<String>) {
    let data = parse_raw_document(source);
    let mut output = String::with_capacity(source.len());
    let mut diagnostics = Vec::new();
    let mut seen_entries = BTreeSet::new();

    for block in &data.blocks {
        match block {
            RawBlock::Whitespace { raw, .. }
            | RawBlock::Comment { raw, .. }
            | RawBlock::Preamble { raw, .. } => output.push_str(raw),
            RawBlock::StringDef { key, span, .. } => {
                push_diagnostic(
                    &mut diagnostics,
                    collect_diagnostics,
                    format!(
                        "ignored string definition {} at {}..{} during literal recovery",
                        quoted(key),
                        span.start,
                        span.end
                    ),
                );
            }
            RawBlock::Entry { id, .. } => {
                if let Some(entry) = data.entry_blocks.get(*id) {
                    if seen_entries.insert(entry.key.clone()) {
                        render_literal_entry(entry, &mut output);
                    } else {
                        push_diagnostic(
                            &mut diagnostics,
                            collect_diagnostics,
                            format!(
                                "ignored duplicate BibTeX entry key {} at {}..{}",
                                quoted(&entry.key),
                                entry.span.start,
                                entry.span.end
                            ),
                        );
                    }
                }
            }
            RawBlock::Failed { error, span, .. } => {
                push_diagnostic(
                    &mut diagnostics,
                    collect_diagnostics,
                    format!(
                        "ignored malformed BibTeX block at {}..{}: {}",
                        span.start, span.end, error
                    ),
                );
            }
            RawBlock::Other { raw, span } => {
                if !raw.trim().is_empty() {
                    push_diagnostic(
                        &mut diagnostics,
                        collect_diagnostics,
                        format!("ignored raw BibTeX text at {}..{}", span.start, span.end),
                    );
                }
            }
        }
    }

    (output, diagnostics)
}

fn render_literal_entry(entry: &RawEntryData, output: &mut String) {
    output.push('@');
    output.push_str(&entry.kind);
    output.push('{');
    output.push_str(&entry.key);
    for field in &entry.field_blocks {
        output.push_str(",\n  ");
        output.push_str(&field.name);
        output.push_str(" = {");
        write_literal_field_value(&field.value, output);
        output.push('}');
    }
    output.push_str("\n}\n");
}

fn write_literal_field_value(value: &str, output: &mut String) {
    for ch in value.chars() {
        if ch == '%' {
            output.push('\\');
        }
        output.push(ch);
    }
}

pub fn remove_block_containing_span(source: &str, span: Range<usize>) -> Option<(String, String)> {
    let data = parse_raw_document(source);
    let block = data
        .blocks
        .iter()
        .filter(|block| {
            !matches!(
                block,
                RawBlock::Whitespace { .. } | RawBlock::Comment { .. }
            )
        })
        .find(|block| block_contains_span(block.span(), &span))
        .or_else(|| {
            data.blocks
                .iter()
                .rev()
                .filter(|block| {
                    !matches!(
                        block,
                        RawBlock::Whitespace { .. } | RawBlock::Comment { .. }
                    )
                })
                .find(|block| block.span().end <= span.start || span.start >= source.len())
        })?;
    let block_span = block.span();
    let mut output = String::with_capacity(source.len().saturating_sub(block_span.len()) + 1);
    output.push_str(&source[..block_span.start]);
    output.push('\n');
    output.push_str(&source[block_span.end..]);
    Some((
        output,
        format!("removed {}", describe_recovered_block(block)),
    ))
}

fn block_contains_span(block: &Range<usize>, span: &Range<usize>) -> bool {
    block.start <= span.start && span.end <= block.end
}

fn describe_recovered_block(block: &RawBlock) -> String {
    match block {
        RawBlock::Preamble { span, .. } => format!("preamble at {}..{}", span.start, span.end),
        RawBlock::StringDef { key, span, .. } => {
            format!(
                "string definition {} at {}..{}",
                quoted(key),
                span.start,
                span.end
            )
        }
        RawBlock::Entry { key, span, .. } => {
            format!("entry {} at {}..{}", quoted(key), span.start, span.end)
        }
        RawBlock::Failed { span, .. } => {
            format!("malformed block at {}..{}", span.start, span.end)
        }
        RawBlock::Other { span, .. } => {
            format!("raw block at {}..{}", span.start, span.end)
        }
        RawBlock::Whitespace { span, .. } => {
            format!("whitespace at {}..{}", span.start, span.end)
        }
        RawBlock::Comment { span, .. } => format!("comment at {}..{}", span.start, span.end),
    }
}
