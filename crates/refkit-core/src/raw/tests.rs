use super::*;

fn raw_field_value(doc: &RawDocument, entry_key: &str, field_key: &str) -> String {
    let entry_id = doc.unique_entry(entry_key).unwrap().unwrap();
    let field_id = doc.unique_field(entry_id, field_key).unwrap().unwrap();
    doc.field_info(entry_id, field_id).unwrap().value
}

#[test]
fn parse_raw_document_preserves_blocks_and_recovers_entries() {
    let source = concat!(
        "% file comment\n",
        "@string{jcs = \"Journal of Citation Systems\"}\n",
        "@preamble{\"A\" # \"B\"}\n",
        "@article{kept,\n",
        "  title = {Kept Title},\n",
        "  journal = jcs # \" Extra\",\n",
        "  year = 2024\n",
        "}\n",
        "@broken{missing,\n",
        "  title = {No close}\n",
        "@article{after,\n",
        "  title = \"After Title\"\n",
        "}\n",
        "trailing text\n",
    );

    let doc = RawDocument::parse(source);
    let blocks = doc.blocks();

    assert_eq!(doc.entry_count(), 2);
    assert!(doc.contains_entry("kept"));
    assert!(doc.contains_entry("after"));
    assert_eq!(doc.comments(), vec!["% file comment\n".to_string()]);
    assert_eq!(
        doc.strings().get("jcs").map(String::as_str),
        Some("Journal of Citation Systems")
    );
    assert_eq!(doc.preamble(), "\"A\" # \"B\"");
    assert!(blocks.iter().any(
        |block| matches!(block, RawBlockInfo::Comment { raw, .. } if raw == "% file comment\n")
    ));
    assert!(
        blocks
            .iter()
            .any(|block| matches!(block, RawBlockInfo::StringDef { key, value, .. } if key == "jcs" && value == "Journal of Citation Systems"))
    );
    assert!(
        doc.failed_blocks()
            .iter()
            .any(|block| matches!(block, RawBlockInfo::Failed { error, .. } if error.contains("closing delimiter")))
    );
    assert!(
        blocks.iter().any(
            |block| matches!(block, RawBlockInfo::Other { raw, .. } if raw == "trailing text\n")
        )
    );
}

#[test]
fn parse_raw_document_keeps_percent_inside_braced_values() {
    let doc = RawDocument::parse(concat!(
        "@article{encoded,\n",
        "  title = {Percent Encoded URL},\n",
        "  url = {https://example.test/path%2Fpaper?partnerID=40},\n",
        "  year = {2024}\n",
        "}\n",
    ));

    assert_eq!(doc.failed_blocks(), Vec::<RawBlockInfo>::new());
    assert_eq!(
        raw_field_value(&doc, "encoded", "url"),
        "https://example.test/path%2Fpaper?partnerID=40"
    );
}

#[test]
fn raw_document_reads_common_value_forms_and_reports_broken_blocks() {
    let doc = RawDocument::parse(concat!(
        "@article{forms,\n",
        "  title = {A {B}},\n",
        "  subtitle = \"C {D}\",\n",
        "  journal = jcs,\n",
        "  note = jcs # \" Extra\"\n",
        "}\n",
        "@article{bad,\n",
        "  title = Bad{Thing}\n",
        "}\n",
        "@article{unclosed,\n",
        "  title = {No close\n",
        "}\n",
    ));

    assert_eq!(raw_field_value(&doc, "forms", "title"), "A {B}");
    assert_eq!(raw_field_value(&doc, "forms", "subtitle"), "C {D}");
    assert_eq!(raw_field_value(&doc, "forms", "journal"), "jcs");
    assert_eq!(raw_field_value(&doc, "forms", "note"), "jcs # \" Extra\"");
    assert!(doc.failed_blocks().iter().any(
        |block| matches!(block, RawBlockInfo::Failed { raw, error, .. } if raw.contains("@article{bad") && error.contains("bare field value"))
    ));
    assert!(doc.failed_blocks().iter().any(
        |block| matches!(block, RawBlockInfo::Failed { raw, error, .. } if raw.contains("@article{unclosed") && error.contains("closing delimiter"))
    ));
}

#[test]
fn sanitizer_keeps_valid_entries_and_reports_failed_or_other_blocks() {
    let source = concat!(
        "prefix text\n",
        "@article{valid,\n",
        "  title = {Valid}\n",
        "}\n",
        "@broken{missing,\n",
        "  title = {No close}\n",
        "@article{after,\n",
        "  title = {After}\n",
        "}\n",
    );

    let (sanitized, diagnostics) = sanitize_biblatex_for_library(source, false, true);

    assert!(sanitized.contains("@article{valid"));
    assert!(sanitized.contains("@article{after"));
    assert!(!sanitized.contains("@broken"));
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics[0].contains("ignored raw BibTeX text"));
    assert!(diagnostics[1].contains("ignored malformed BibTeX block"));
}

#[test]
fn sanitizer_drops_later_duplicate_entries_with_diagnostics() {
    let source = concat!(
        "@article{same,\n",
        "  title = {First}\n",
        "}\n",
        "@article{same,\n",
        "  title = {Second}\n",
        "}\n",
    );

    let (sanitized, diagnostics) = sanitize_biblatex_for_library(source, false, true);

    assert!(sanitized.contains("title = {First}"));
    assert!(!sanitized.contains("title = {Second}"));
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].contains("ignored duplicate BibTeX entry key \"same\""));
}

#[test]
fn literal_sanitizer_escapes_percent_encoded_values() {
    let (sanitized, diagnostics) = sanitize_biblatex_for_library_literals(
        concat!(
            "@article{encoded,\n",
            "  title = {Percent Encoded URL},\n",
            "  url = {https://example.test/path%2Fpaper},\n",
            "}\n",
        ),
        true,
    );

    assert!(diagnostics.is_empty());
    assert!(sanitized.contains("url = {https://example.test/path\\%2Fpaper}"));
}

#[test]
fn render_document_patches_only_changed_fields() {
    let mut doc = RawDocument::parse(concat!(
        "@article{patch,\n",
        "  braced = {Old},\n",
        "  quoted = \"Old\",\n",
        "  bare = old,\n",
        "  expr = macro # \"Old\"\n",
        "}\n",
    ));
    let entry_id = doc.unique_entry("patch").unwrap().unwrap();
    for (key, value) in [
        ("braced", "New"),
        ("quoted", "Quoted"),
        ("bare", "Bare Value"),
        ("expr", "macro # \"New\""),
    ] {
        let field_id = doc.unique_field(entry_id, key).unwrap().unwrap();
        doc.set_field_value(entry_id, field_id, value.to_string())
            .unwrap();
    }

    let rendered = doc.render().unwrap();

    assert!(rendered.contains("braced = {New}"));
    assert!(rendered.contains("quoted = \"Quoted\""));
    assert!(rendered.contains("bare = {Bare Value}"));
    assert!(rendered.contains("expr = {macro # \"New\"}"));
}

#[test]
fn duplicate_field_parse_preserves_ordered_occurrences() {
    let mut doc = RawDocument::parse(concat!(
        "@article{duplicate,\n",
        "  title = {First},\n",
        "  TITLE = {Second},\n",
        "  year = {2024}\n",
        "}\n",
    ));

    let entry_id = doc.unique_entry("duplicate").unwrap().unwrap();
    let title_fields = doc.fields_for_key(entry_id, "title").unwrap();
    assert_eq!(title_fields.len(), 2);
    assert_eq!(title_fields[0].value, "First");
    assert_eq!(title_fields[1].value, "Second");
    assert!(doc.unique_field(entry_id, "title").is_err());
    doc.set_field_value(entry_id, title_fields[1].id, "Edited".to_string())
        .unwrap();

    let rendered = doc.render().unwrap();

    assert!(rendered.contains("title = {First}"));
    assert!(rendered.contains("TITLE = {Edited}"));
    assert!(!rendered.contains("TITLE = {Second}"));
}

#[test]
fn raw_document_rejects_unsafe_field_edits() {
    let mut doc = RawDocument::parse(concat!(
        "@article{editable,\n",
        "  title = {Old Title},\n",
        "  subtitle = \"Old Subtitle\",\n",
        "  note = jcs # \" Extra\"\n",
        "}\n",
    ));
    let entry_id = doc.unique_entry("editable").unwrap().unwrap();
    let title_id = doc.unique_field(entry_id, "title").unwrap().unwrap();
    let subtitle_id = doc.unique_field(entry_id, "subtitle").unwrap().unwrap();
    let note_id = doc.unique_field(entry_id, "note").unwrap().unwrap();

    assert!(
        doc.set_field_value(entry_id, title_id, "{NASA} Mission".to_string())
            .is_ok()
    );
    assert!(
        doc.set_field_value(entry_id, title_id, "bad%value".to_string())
            .is_err()
    );
    assert!(
        doc.set_field_value(entry_id, title_id, "bad\nvalue".to_string())
            .is_err()
    );
    assert!(
        doc.set_field_value(entry_id, title_id, "bad\\".to_string())
            .is_err()
    );
    assert!(
        doc.set_field_value(entry_id, subtitle_id, "bad\"value".to_string())
            .is_err()
    );
    assert!(
        doc.set_field_value(entry_id, subtitle_id, "bad%value".to_string())
            .is_err()
    );
    assert!(
        doc.set_field_value(entry_id, note_id, "{unbalanced".to_string())
            .is_err()
    );

    assert_eq!(
        doc.field_info(entry_id, title_id).unwrap().value,
        "{NASA} Mission"
    );
    assert_eq!(
        doc.field_info(entry_id, subtitle_id).unwrap().value,
        "Old Subtitle"
    );
    assert_eq!(
        doc.field_info(entry_id, note_id).unwrap().value,
        "jcs # \" Extra\""
    );
}
