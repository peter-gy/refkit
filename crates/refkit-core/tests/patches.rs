//! Atomic source patches, occurrence mappings, and byte-preservation contracts.

#![cfg(test)]

use refkit_core::{BibEdit, BibFieldValue, BibPatchErrorCode as Code, BibPatchKind, RawDocument};

#[test]
fn patch_applies_all_structural_edits_and_preserves_original_bytes() {
    let source = "% é preface\n@Book(a, title = {Old}, note={Remove}, year=2024)\n% between\n@misc{b,title={B}}\n% tail\n";
    let original = RawDocument::parse(source);
    let a = original.unique_entry("a").unwrap().unwrap();
    let b = original.unique_entry("b").unwrap().unwrap();
    let title = original.unique_field(a, "title").unwrap().unwrap();
    let note = original.unique_field(a, "note").unwrap().unwrap();
    let result = original
        .apply_patch(&[
            BibEdit::SetField {
                expression: false,
                entry_id: a,
                field_id: title,
                value: "Longer {Title}".into(),
            },
            BibEdit::RemoveField {
                entry_id: a,
                field_id: note,
            },
            BibEdit::AddField {
                expression: false,
                entry_id: a,
                name: "doi".into(),
                value: "10.1234/a".into(),
            },
            BibEdit::RenameEntry {
                entry_id: a,
                key: "renamed".into(),
            },
            BibEdit::SetEntryType {
                entry_id: a,
                entry_type: "article".into(),
            },
            BibEdit::RemoveEntry { entry_id: b },
            BibEdit::AddEntry {
                key: "new".into(),
                entry_type: "book".into(),
                fields: vec![BibFieldValue {
                    expression: false,
                    name: "title".into(),
                    value: "New Work".into(),
                }],
                before: Some(a),
            },
        ])
        .unwrap();
    let output = result.document.render().unwrap();
    assert_eq!(original.render().unwrap(), source);
    assert_eq!(original.field_info(a, title).unwrap().value, "Old");
    assert_eq!(result.document.entry_keys(), ["new", "renamed"]);
    assert!(output.contains("@article(renamed, title = {Longer {Title}},  year=2024"));
    assert!(output.contains("% between\n\n% tail\n"));
    assert_eq!(result.entries[0].before.as_ref().unwrap().id.index(), 0);
    assert_eq!(result.entries[0].after.as_ref().unwrap().id.index(), 1);
    assert!(result.entries[0].fields[1].after.is_none());
    assert!(result.entries[1].after.is_none());
    assert!(result.entries[2].before.is_none());
    let mut old_cursor = 0;
    let mut new_cursor = 0;
    for change in &result.changes {
        assert_eq!(
            &source[old_cursor..change.before.start],
            &output[new_cursor..change.after.start]
        );
        old_cursor = change.before.end;
        new_cursor = change.after.end;
    }
    assert_eq!(&source[old_cursor..], &output[new_cursor..]);
}

#[test]
fn patch_conflicts_and_invalid_values_leave_original_unchanged() {
    let source = "@book{a,title={Original}}";
    let document = RawDocument::parse(source);
    let entry = document.unique_entry("a").unwrap().unwrap();
    let field = document.unique_field(entry, "title").unwrap().unwrap();
    let set = BibEdit::SetField {
        expression: false,
        entry_id: entry,
        field_id: field,
        value: "Edited".into(),
    };
    for patch in [
        vec![set.clone(), set.clone()],
        vec![
            set.clone(),
            BibEdit::RemoveField {
                entry_id: entry,
                field_id: field,
            },
        ],
        vec![set, BibEdit::RemoveEntry { entry_id: entry }],
    ] {
        assert_eq!(
            document.apply_patch(&patch).unwrap_err().code,
            Code::Overlap
        );
    }
    assert_eq!(
        document
            .apply_patch(&[BibEdit::SetField {
                expression: false,
                entry_id: entry,
                field_id: field,
                value: "bad}value".into()
            }])
            .unwrap_err()
            .code,
        Code::InvalidValue
    );
    assert_eq!(document.render().unwrap(), source);
}

#[test]
fn renames_rewrite_unambiguous_references_with_shared_key_encoding() {
    let source = "@string{parent={Old}}\n@book{Old,title={Parent}}\n@misc{child,crossref=parent,xdata={Old, other}}\n@misc{other,title={Other}}";
    let document = RawDocument::parse(source);
    let entry = document.unique_entry("Old").unwrap().unwrap();
    let result = document
        .apply_patch(&[BibEdit::RenameEntry {
            entry_id: entry,
            key: "New_Key".into(),
        }])
        .unwrap();
    let output = result.document.render().unwrap();
    assert!(output.contains("@string{parent={Old}}"));
    let resolved = result.document.resolve().unwrap();
    assert!(resolved[1].fields["crossref"].contains("New"));
    assert_eq!(
        result
            .changes
            .iter()
            .filter(|change| change.kind == BibPatchKind::RewriteReference)
            .count(),
        2
    );
    assert_eq!(document.render().unwrap(), source);
}

#[test]
fn ambiguous_reference_renames_fail_but_explicit_reference_edits_are_final() {
    let source = "@book{a,title={First}}@book{a,title={Second}}@misc{child,crossref={a}}";
    let document = RawDocument::parse(source);
    let entries = document.entry_occurrences();
    let rename = BibEdit::RenameEntry {
        entry_id: entries[0].id,
        key: "new".into(),
    };
    assert_eq!(
        document
            .apply_patch(std::slice::from_ref(&rename))
            .unwrap_err()
            .code,
        Code::AmbiguousReference
    );
    let field = document
        .unique_field(entries[2].id, "crossref")
        .unwrap()
        .unwrap();
    let result = document
        .apply_patch(&[
            rename,
            BibEdit::SetField {
                expression: false,
                entry_id: entries[2].id,
                field_id: field,
                value: "new".into(),
            },
        ])
        .unwrap();
    assert!(result.document.render().unwrap().contains("crossref={new}"));
}

#[test]
fn field_authoring_handles_empty_entries_comments_and_duplicate_occurrences() {
    for source in [
        "@book{a}",
        "@book{a,}",
        "@book{a,title={A}% keep\n}",
        "@book(a, title={A},\n)",
    ] {
        let document = RawDocument::parse(source);
        let entry = document.entry_occurrences()[0].id;
        let result = document
            .apply_patch(&[
                BibEdit::AddField {
                    expression: false,
                    entry_id: entry,
                    name: "note".into(),
                    value: "One".into(),
                },
                BibEdit::AddField {
                    expression: false,
                    entry_id: entry,
                    name: "NOTE".into(),
                    value: "Two".into(),
                },
            ])
            .unwrap();
        assert_eq!(
            result
                .document
                .fields_for_key(result.document.entry_occurrences()[0].id, "note")
                .unwrap()
                .len(),
            2
        );
        assert_eq!(result.warnings[0].code, "duplicate_field");
        assert_eq!(document.render().unwrap(), source);
    }
}

#[test]
fn insert_before_removed_anchor_is_snapshot_relative() {
    let source = "@book{a,title={A}}@book{b,title={B}}";
    let document = RawDocument::parse(source);
    let a = document.entry_occurrences()[0].id;
    let result = document
        .apply_patch(&[
            BibEdit::RemoveEntry { entry_id: a },
            BibEdit::AddEntry {
                key: "first".into(),
                entry_type: "book".into(),
                fields: vec![],
                before: Some(a),
            },
            BibEdit::AddEntry {
                key: "second".into(),
                entry_type: "book".into(),
                fields: vec![],
                before: Some(a),
            },
        ])
        .unwrap();
    assert_eq!(result.document.entry_keys(), ["first", "second", "b"]);
    assert_eq!(result.entries[1].after.as_ref().unwrap().id.index(), 2);
}

#[test]
fn authoring_a_key_and_fields_in_an_empty_entry_is_order_independent() {
    for source in ["@book{}", "@book{,}"] {
        let document = RawDocument::parse(source);
        let entry = document.entry_occurrences()[0].id;
        let operations = [
            BibEdit::AddField {
                expression: false,
                entry_id: entry,
                name: "title".into(),
                value: "Title".into(),
            },
            BibEdit::RenameEntry {
                entry_id: entry,
                key: "new".into(),
            },
        ];
        let result = document.apply_patch(&operations).unwrap();
        let reversed = document
            .apply_patch(&operations.into_iter().rev().collect::<Vec<_>>())
            .unwrap();
        assert_eq!(
            result.document.render().unwrap(),
            reversed.document.render().unwrap()
        );
        assert_eq!(
            result.document.resolve().unwrap()[0].fields["title"],
            "Title"
        );
    }
}

#[test]
fn edits_preserve_unrelated_malformed_blocks_and_can_reduce_oversized_source() {
    let source = "@broken\n@book{a,title={Old}}\n";
    let document = RawDocument::parse(source);
    let entry = document.entry_occurrences()[0].id;
    let field = document.unique_field(entry, "title").unwrap().unwrap();
    let result = document
        .apply_patch(&[BibEdit::SetField {
            expression: false,
            entry_id: entry,
            field_id: field,
            value: "New".into(),
        }])
        .unwrap();
    assert!(result.document.render().unwrap().starts_with("@broken\n"));
    let oversized = RawDocument::parse(&format!(
        "@book{{a,title={{{}}}}}",
        "x".repeat(16 * 1024 * 1024)
    ));
    let entry = oversized.entry_occurrences()[0].id;
    let field = oversized.unique_field(entry, "title").unwrap().unwrap();
    assert!(
        oversized
            .apply_patch(&[BibEdit::SetField {
                expression: false,
                entry_id: entry,
                field_id: field,
                value: "Small".into()
            }])
            .is_ok()
    );
}

#[test]
fn patch_resource_limits_reject_before_returning_an_oversized_snapshot() {
    let document = RawDocument::parse("@book{a,title={Old}}");
    let entry = document.entry_occurrences()[0].id;
    let field = document.unique_field(entry, "title").unwrap().unwrap();
    let operations = vec![BibEdit::RemoveEntry { entry_id: entry }; 100_001];
    assert_eq!(
        document.apply_patch(&operations).unwrap_err().code,
        Code::ResourceLimit
    );
    for size in [16 * 1024 * 1024, 16 * 1024 * 1024 + 1] {
        let operation = BibEdit::SetField {
            expression: false,
            entry_id: entry,
            field_id: field,
            value: "x".repeat(size),
        };
        assert_eq!(
            document.apply_patch(&[operation]).unwrap_err().code,
            Code::ResourceLimit
        );
    }
    assert_eq!(document.render().unwrap(), "@book{a,title={Old}}");
}

#[test]
fn literal_patches_preserve_padding_when_inspection_trims_it() {
    let document = RawDocument::parse("@book{a,title={0}}");
    let entry = document.entry_occurrences()[0].id;
    let field = document.unique_field(entry, "title").unwrap().unwrap();
    let result = document
        .apply_patch(&[BibEdit::SetField {
            entry_id: entry,
            field_id: field,
            value: " ".into(),
            expression: false,
        }])
        .unwrap();
    assert_eq!(result.document.render().unwrap(), "@book{a,title={ }}");
    assert_eq!(result.document.field_info(entry, field).unwrap().value, "");
    assert_eq!(result.document.resolve().unwrap()[0].fields["title"], " ");
}
