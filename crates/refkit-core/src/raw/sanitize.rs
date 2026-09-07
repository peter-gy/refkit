use std::collections::HashSet;

use super::{RawBlock, parse_raw_document};
use crate::library::{Diagnostic, DiagnosticAction};

pub(crate) fn sanitize_biblatex_for_library(source: &str) -> (String, Vec<Diagnostic>) {
    let data = parse_raw_document(source);
    let mut output = source.as_bytes().to_vec();
    let mut diagnostics = Vec::new();
    let mut keys = HashSet::new();
    let mut comment_end = 0;
    for block in &data.blocks {
        let span = block.span().clone();
        if let RawBlock::Comment { raw, .. } = block
            && raw.starts_with('%')
        {
            comment_end = source[span.start..]
                .find('\n')
                .map_or(source.len(), |offset| span.start + offset + 1);
        }
        if span.start < comment_end {
            continue;
        }
        let (code, message, entry) = match block {
            RawBlock::Entry { key, .. } if !keys.insert(key.clone()) => (
                "duplicate_key",
                format!("ignored duplicate BibTeX entry key {key:?}"),
                Some(key.clone()),
            ),
            RawBlock::Failed { error, .. } => (
                "malformed_block",
                format!("ignored malformed BibTeX block: {error}"),
                None,
            ),
            RawBlock::Other { raw, .. } if !raw.trim().is_empty() => {
                ("raw_text", "ignored raw BibTeX text".to_string(), None)
            }
            _ => continue,
        };
        for byte in &mut output[span.clone()] {
            if *byte != b'\n' && *byte != b'\r' {
                *byte = b' ';
            }
        }
        let mut diagnostic =
            Diagnostic::error(code, Some(span), message).recovered(DiagnosticAction::DroppedBlock);
        diagnostic.entry = entry;
        diagnostics.push(diagnostic);
    }
    (
        String::from_utf8(output).expect("masking complete UTF-8 spans preserves UTF-8"),
        diagnostics,
    )
}
