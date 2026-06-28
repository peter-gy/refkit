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
fn parse_raw_document_keeps_escaped_closing_brace_inside_braced_values() {
    let doc = RawDocument::parse(concat!(
        "@article{escaped,\n",
        "  title = {A \\} B},\n",
        "  year = {2026}\n",
        "}\n",
    ));

    assert_eq!(doc.failed_blocks(), Vec::<RawBlockInfo>::new());
    assert_eq!(raw_field_value(&doc, "escaped", "title"), "A \\} B");
    assert_eq!(raw_field_value(&doc, "escaped", "year"), "2026");
}

#[test]
fn parse_raw_document_keeps_terminal_escaped_closing_brace_inside_braced_values() {
    let doc = RawDocument::parse(concat!(
        "@article{escaped,\n",
        "  title = {A \\}},\n",
        "  year = {2026}\n",
        "}\n",
    ));

    assert_eq!(doc.failed_blocks(), Vec::<RawBlockInfo>::new());
    assert_eq!(raw_field_value(&doc, "escaped", "title"), "A \\}");
    assert_eq!(raw_field_value(&doc, "escaped", "year"), "2026");
}

#[test]
fn raw_document_exposes_shared_syntax_records_for_formatters() {
    let doc = RawDocument::parse(concat!(
        "% file comment\n",
        "@article{format,\n",
        "  title = {Shared Syntax},\n",
        "  year = 2026\n",
        "}\n",
    ));

    let syntax = doc.syntax();

    assert_eq!(syntax.entries.len(), 1);
    assert_eq!(syntax.entries[0].key, "format");
    assert_eq!(syntax.entries[0].kind, "article");
    assert_eq!(syntax.entries[0].fields.len(), 2);
    assert_eq!(syntax.entries[0].fields[0].name, "title");
    assert_eq!(syntax.entries[0].fields[0].value_mode, RawValueMode::Braced);
    assert!(syntax.blocks.iter().any(
        |block| matches!(block, RawSyntaxBlock::Comment { raw, .. } if raw == "% file comment\n")
    ));
    assert!(
        syntax
            .blocks
            .iter()
            .any(|block| matches!(block, RawSyntaxBlock::Entry { key, .. } if key == "format"))
    );
}

#[test]
fn raw_document_parses_missing_key_entries_for_formatter_warnings() {
    let doc = RawDocument::parse("@article{\n  title = {No key}\n}\n");
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert_eq!(syntax.entries.len(), 1);
    assert_eq!(syntax.entries[0].key, "");
    assert_eq!(syntax.entries[0].fields[0].name, "title");
}

#[test]
fn raw_document_preserves_case_and_strange_field_names_for_formatters() {
    let doc = RawDocument::parse(concat!(
        "@ARTICLE{Key,\n",
        "  Number = {1},\n",
        "  weird-key = {A},\n",
        "  _#bo = {B},\n",
        "  key with spaces = thing\n",
        "}\n",
    ));
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert_eq!(syntax.entries[0].kind, "ARTICLE");
    assert_eq!(syntax.entries[0].fields[0].name, "Number");
    assert_eq!(syntax.entries[0].fields[1].name, "weird-key");
    assert_eq!(syntax.entries[0].fields[2].name, "_#bo");
    assert_eq!(syntax.entries[0].fields[3].name, "key with spaces");
    assert!(doc.contains_field(syntax.entries[0].id, "NUMBER"));
    assert!(doc.contains_field(syntax.entries[0].id, "key with spaces"));
}

#[test]
fn raw_document_trims_tabbed_field_name_whitespace() {
    let doc = RawDocument::parse(concat!(
        "@customd{LDN3,\n",
        "  OPTIONS     = {labelalphanametemplatename=customd},\n",
        "  AUTHOR\t    = {Vela, Luis and given={Ura Ru}, family={Juan}},\n",
        "}\n",
    ));
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert_eq!(syntax.entries[0].fields[0].name, "OPTIONS");
    assert_eq!(syntax.entries[0].fields[1].name, "AUTHOR");
}

#[test]
fn raw_document_preserves_assignmentless_string_blocks() {
    let doc = RawDocument::parse("@string{fooobaar}\n@article{foo,title={Foo}}\n");
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert!(doc.strings().is_empty());
    assert!(syntax.blocks.iter().any(
        |block| matches!(block, RawSyntaxBlock::StringDef { raw, key, value, .. } if raw == "@string{fooobaar}" && key.is_empty() && value.is_empty())
    ));
    assert_eq!(syntax.entries.len(), 1);
}

#[test]
fn raw_document_parses_valueless_fields_for_formatter_transforms() {
    let doc = RawDocument::parse("@article{empty,\n  day,\n  year = {}\n}\n");
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert_eq!(syntax.entries[0].fields[0].name, "day");
    assert_eq!(syntax.entries[0].fields[0].value, "");
    assert_eq!(
        syntax.entries[0].fields[0].value_mode,
        RawValueMode::Missing
    );
    assert_eq!(syntax.entries[0].fields[1].value, "");
    assert_eq!(syntax.entries[0].fields[1].value_mode, RawValueMode::Braced);
}

#[test]
fn raw_document_rejects_non_empty_edits_to_valueless_fields() {
    let mut doc = RawDocument::parse("@article{empty,\n  day,\n}\n");
    let entry_id = doc.unique_entry("empty").unwrap().unwrap();
    let field_id = doc.unique_field(entry_id, "day").unwrap().unwrap();

    let error = doc
        .set_field_value(entry_id, field_id, "12".to_string())
        .unwrap_err();

    assert!(
        matches!(error, RawEditError::InvalidValue(message) if message.contains("without an assignment"))
    );
    assert_eq!(doc.render().unwrap(), "@article{empty,\n  day,\n}\n");
}

#[test]
fn raw_document_parses_empty_entries_without_key_separator() {
    let doc = RawDocument::parse("@misc{emptyref2\n\n}\n");
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert_eq!(syntax.entries.len(), 1);
    assert_eq!(syntax.entries[0].key, "emptyref2");
    assert!(syntax.entries[0].fields.is_empty());
}

#[test]
fn raw_document_recovers_entry_after_invalid_at_text() {
    let doc = RawDocument::parse("@blah @article{foo,foo=bar}\nfoo@blah @article{bar,foo=bar}\n");
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert_eq!(syntax.entries.len(), 2);
    assert_eq!(syntax.entries[0].key, "foo");
    assert_eq!(syntax.entries[1].key, "bar");
    assert!(
        syntax
            .blocks
            .iter()
            .any(|block| matches!(block, RawSyntaxBlock::Other { raw, .. } if raw == "@blah "))
    );
    assert!(
        syntax
            .blocks
            .iter()
            .any(|block| matches!(block, RawSyntaxBlock::Other { raw, .. } if raw == "foo@blah "))
    );
}

#[test]
fn raw_document_treats_line_style_comment_as_comment_block() {
    let doc = RawDocument::parse("@comment blah, blah\n@article{foo,foo=bar}\n");
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert_eq!(syntax.entries.len(), 1);
    assert!(syntax.blocks.iter().any(
        |block| matches!(block, RawSyntaxBlock::Comment { raw, .. } if raw == "@comment blah, blah\n")
    ));
}

#[test]
fn raw_document_treats_line_style_comment_with_braces_as_comment_block() {
    let doc = RawDocument::parse(concat!(
        "@Comment         on Automated Planning and Scheduling, {ICAPS} 2006,\n",
        "@article{foo,foo=bar}\n",
    ));
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert_eq!(syntax.entries.len(), 1);
    assert!(syntax.blocks.iter().any(
        |block| matches!(block, RawSyntaxBlock::Comment { raw, .. } if raw == "@Comment         on Automated Planning and Scheduling, {ICAPS} 2006,\n")
    ));
}

#[test]
fn raw_document_keeps_real_comment_blocks_with_spaced_openers() {
    let doc = RawDocument::parse("@comment {real comment block}\n@article{foo,foo=bar}\n");
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert_eq!(syntax.entries.len(), 1);
    assert!(syntax.blocks.iter().any(
        |block| matches!(block, RawSyntaxBlock::Comment { raw, .. } if raw == "@comment {real comment block}")
    ));
}

#[test]
fn raw_document_keeps_line_style_comment_words_inside_text() {
    let doc = RawDocument::parse("plain prose with @Comment entry type.\n@article{foo,foo=bar}\n");
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert!(syntax.blocks.iter().any(
        |block| matches!(block, RawSyntaxBlock::Other { raw, .. } if raw == "plain prose with @Comment entry type.\n")
    ));
    assert_eq!(syntax.entries.len(), 1);
}

#[test]
fn raw_document_recovers_unicode_at_text_without_byte_boundary_panic() {
    let doc = RawDocument::parse("@ṣource without opener\n@article{foo,foo=bar}\n");
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert_eq!(syntax.entries.len(), 1);
    assert!(syntax.blocks.iter().any(
        |block| matches!(block, RawSyntaxBlock::Other { raw, .. } if raw == "@ṣource without opener\n")
    ));
}

#[test]
fn raw_document_counts_backslashed_braces_inside_braced_values() {
    let doc = RawDocument::parse(concat!(
        "@article{escaped,\n",
        "  isbn = {{1938-7849|escape{\\}}},\n",
        "  issn = {1938-7849}\n",
        "}\n",
    ));
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert_eq!(syntax.entries.len(), 1);
    assert_eq!(
        raw_field_value(&doc, "escaped", "isbn"),
        "{1938-7849|escape{\\}}"
    );
    assert_eq!(raw_field_value(&doc, "escaped", "issn"), "1938-7849");
}

#[test]
fn raw_document_recovers_entry_after_inline_percent_comment() {
    let doc = RawDocument::parse("% another comment @article{foo,foo=bar}\n");
    let syntax = doc.syntax();

    assert!(doc.failed_blocks().is_empty());
    assert_eq!(syntax.entries.len(), 1);
    assert_eq!(syntax.entries[0].key, "foo");
    assert!(syntax.blocks.iter().any(
        |block| matches!(block, RawSyntaxBlock::Comment { raw, .. } if raw == "% another comment ")
    ));
}

#[test]
fn raw_document_rejects_latex_special_characters_in_entry_keys() {
    for source in [
        "@article{invalid{,\n  title={Foo}\n}",
        "@article{invalid$,\n  title={Foo}\n}",
        "@article{invalid%,\n  title={Foo}\n}",
        "@article{invalid#,\n  title={Foo}\n}",
        "@article{invalid~,\n  title={Foo}\n}",
    ] {
        let doc = RawDocument::parse(source);
        assert!(
            doc.failed_blocks()
                .iter()
                .any(|block| matches!(block, RawBlockInfo::Failed { error, .. } if error.contains("entry key"))),
            "expected invalid entry key for {source}"
        );
    }
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
fn raw_document_exposes_expression_atoms_for_formatter_consumers() {
    let doc = RawDocument::parse(concat!(
        "@article{expr,\n",
        "  author = aubert # \" and \" # Varacca,\n",
        "  journal = abc # { 123 }\n",
        "}\n",
    ));
    let syntax = doc.syntax();
    let author = syntax.entries[0]
        .fields
        .iter()
        .find(|field| field.name == "author")
        .unwrap();
    let journal = syntax.entries[0]
        .fields
        .iter()
        .find(|field| field.name == "journal")
        .unwrap();

    assert_eq!(author.value_mode, RawValueMode::Expression);
    assert_eq!(
        author
            .value_atoms
            .iter()
            .map(|atom| (atom.value.as_str(), atom.value_mode))
            .collect::<Vec<_>>(),
        vec![
            ("aubert", RawValueMode::Bare),
            (" and ", RawValueMode::Quoted),
            ("Varacca", RawValueMode::Bare),
        ]
    );
    assert_eq!(
        journal
            .value_atoms
            .iter()
            .map(|atom| (atom.value.as_str(), atom.value_mode))
            .collect::<Vec<_>>(),
        vec![("abc", RawValueMode::Bare), (" 123 ", RawValueMode::Braced),]
    );
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
fn sanitizer_ignores_entries_recovered_from_percent_comments_for_duplicates() {
    let source = concat!(
        "% @article{same,title={Commented}}\n",
        "@article{same,\n",
        "  title = {Real}\n",
        "}\n",
    );

    let (sanitized, diagnostics) = sanitize_biblatex_for_library(source, false, true);

    assert!(sanitized.contains("% \n"));
    assert!(sanitized.contains("title = {Real}"));
    assert!(!sanitized.contains("Commented"));
    assert!(diagnostics.is_empty());
}

#[test]
fn sanitizer_ignores_multiple_entries_recovered_from_one_percent_comment_line() {
    let source = concat!(
        "% @article{first,title={Hidden A}} @article{second,title={Hidden B}}\n",
        "@article{first,\n",
        "  title = {Real A}\n",
        "}\n",
        "@article{second,\n",
        "  title = {Real B}\n",
        "}\n",
    );

    let (sanitized, diagnostics) = sanitize_biblatex_for_library(source, false, true);

    assert!(sanitized.contains("title = {Real A}"));
    assert!(sanitized.contains("title = {Real B}"));
    assert!(!sanitized.contains("Hidden A"));
    assert!(!sanitized.contains("Hidden B"));
    assert!(diagnostics.is_empty());
}

#[test]
fn sanitizer_ignores_non_entry_blocks_recovered_from_percent_comments() {
    let source = concat!(
        "% @string{hidden = \"Hidden\"} @preamble{\"Hidden\"} @comment{Hidden} @article{hidden,title={Hidden}}\n",
        "@article{real,\n",
        "  title = hidden\n",
        "}\n",
    );

    let (sanitized, diagnostics) = sanitize_biblatex_for_library(source, false, true);

    assert!(sanitized.contains("@article{real"));
    assert!(sanitized.contains("title = hidden"));
    assert!(!sanitized.contains("@string{hidden"));
    assert!(!sanitized.contains("@preamble"));
    assert!(!sanitized.contains("@comment"));
    assert!(!sanitized.contains("title={Hidden}"));
    assert!(diagnostics.is_empty());
}

#[test]
fn sanitizer_preserves_newline_after_recovered_percent_comment_blocks() {
    let source = concat!(
        "% @article{hidden,title={Hidden}} trailing text\n",
        "@article{real,\n",
        "  title = {Real}\n",
        "}\n",
    );

    let (sanitized, diagnostics) = sanitize_biblatex_for_library(source, false, true);

    assert!(sanitized.starts_with("% \n@article{real"));
    assert!(sanitized.contains("title = {Real}"));
    assert!(!sanitized.contains("Hidden"));
    assert!(!sanitized.contains("trailing text"));
    assert!(diagnostics.is_empty());
}

#[test]
fn sanitizer_ignores_second_percent_tail_after_recovered_comment_blocks() {
    let source = concat!(
        "% @article{hidden,title={Hidden}} % tail\n",
        "@article{real,\n",
        "  title = {Real}\n",
        "}\n",
    );

    let (sanitized, diagnostics) = sanitize_biblatex_for_library(source, false, true);

    assert!(sanitized.starts_with("% \n@article{real"));
    assert!(sanitized.contains("title = {Real}"));
    assert!(!sanitized.contains("Hidden"));
    assert!(!sanitized.contains("tail"));
    assert!(diagnostics.is_empty());
}

#[test]
fn literal_sanitizer_ignores_entries_recovered_from_percent_comments_for_duplicates() {
    let source = concat!(
        "% @article{same,title={Commented}}\n",
        "@article{same,\n",
        "  title = {Real}\n",
        "}\n",
    );

    let (sanitized, diagnostics) = sanitize_biblatex_for_library_literals(source, true);

    assert!(sanitized.contains("% \n"));
    assert!(sanitized.contains("title = {Real}"));
    assert!(!sanitized.contains("Commented"));
    assert!(diagnostics.is_empty());
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
