//! Portable capability boundaries and normalized record projections.

#![cfg(test)]

use refkit_core::{EntryField, Library, RecoveryPolicy};

const BIBLATEX: &str = include_str!("../../../testdata/contracts/biblatex-input.bib");

#[test]
fn biblatex_input_normalizes_through_the_core_port() {
    let library = Library::parse_biblatex(BIBLATEX, RecoveryPolicy::Error).unwrap();
    let entry = &library.records()[0];

    assert_eq!(library.keys(), &["extended-name"]);
    assert_eq!(entry.field(EntryField::Date).as_deref(), Some("2026-02"));
    assert_eq!(
        entry.field(refkit_core::EntryField::Title).as_deref(),
        Some("Typed Bibliography Ports")
    );
}
