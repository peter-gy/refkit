//! Borrowed inspection contracts for immutable raw snapshots.

#![cfg(test)]

use refkit_core::{BibEdit, RawDocument};

#[test]
fn occurrence_indices_follow_source_order_including_duplicates() {
    let document = RawDocument::parse("% comment\n@book{same,title={First}}\n@misc{same}");
    let first = document.entry_id_at(0).unwrap();
    let second = document.entry_id_at(1).unwrap();
    assert_eq!(document.entry_info(first).unwrap().kind, "book");
    assert_eq!(document.entry_info(second).unwrap().kind, "misc");
    assert_ne!(first, second);
    assert!(document.entry_id_at(2).is_none());
    assert!(document.entry_id_at(usize::MAX).is_none());
    assert!(RawDocument::parse("").entry_id_at(0).is_none());

    let field = document.unique_field(first, "title").unwrap().unwrap();
    assert!(document.field_view(second, field).is_none());
    assert!(RawDocument::parse("").field_view(first, field).is_none());
}

#[test]
fn borrowed_fields_retain_original_contents_after_patching() {
    let source = "@book{a,title={Café},title={Second}}";
    let document = RawDocument::parse(source);
    let entry = document.entry_id_at(0).unwrap();
    let fields = document.field_occurrences(entry).unwrap();
    let field = fields[0].id;
    let view = document.field_view(entry, field).unwrap();
    let owned = document.field_info(entry, field).unwrap();
    assert_eq!(view.id, owned.id);
    assert_eq!(view.name, owned.name);
    assert_eq!(view.value, owned.value);
    assert_eq!(view.span, &owned.span);
    assert!(source.get(view.span.clone()).unwrap().contains("Café"));

    let patched = document
        .apply_patch(&[BibEdit::SetField {
            entry_id: entry,
            field_id: field,
            value: "Updated".to_owned(),
            expression: false,
        }])
        .unwrap();
    assert_eq!(view.value, "Café");
    assert_eq!(document.render().unwrap(), source);
    assert_eq!(
        patched.document.field_view(entry, field).unwrap().value,
        "Updated"
    );
    assert_eq!(
        document.field_view(entry, fields[1].id).unwrap().value,
        "Second"
    );
}
