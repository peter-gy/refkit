use refkit_core::{DuplicateRule, MergeStrategy, RawDocument, TidyError, TidyOptions, tidy_bibtex};

#[test]
fn generated_keys_reserve_final_bases_and_retained_original_keys() {
    let result = tidy_bibtex(
        "@misc{first,title={x}}\n@misc{second,title={x}}\n@misc{third,title={xa}}\n@misc{xb,note={Keep key}}",
        TidyOptions {
            generate_keys: Some("[fulltitle:required:lower]".into()),
            ..TidyOptions::default()
        },
    ).unwrap();
    let doc = RawDocument::parse(&result.bibtex);
    assert_eq!(doc.entry_keys(), ["xc", "xd", "xa", "xb"]);
    let renames = result
        .renames
        .iter()
        .map(|rename| {
            (
                rename.entry_id.index(),
                rename.old_key.as_str(),
                rename.new_key.as_str(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        renames,
        [(0, "first", "xc"), (1, "second", "xd"), (2, "third", "xa")]
    );
}

#[test]
fn generated_letter_suffixes_extend_past_one_letter() {
    let source = (0..28)
        .map(|index| format!("@misc{{k{index},title={{Shared}}}}\n"))
        .collect::<String>();
    let result = tidy_bibtex(
        &source,
        TidyOptions {
            generate_keys: Some("[fulltitle:lower]".into()),
            ..TidyOptions::default()
        },
    )
    .unwrap();
    let doc = RawDocument::parse(&result.bibtex);
    assert_eq!(doc.entry_count(), 28);
    assert_eq!(&doc.entry_keys()[25..], ["sharedz", "sharedaa", "sharedab"]);
}

#[test]
fn generated_keys_follow_merged_payload_and_sort_final_keys() {
    let result = tidy_bibtex(
        "@article{z,title={Old},doi={10.1/same}}\n@book{a,title={Zulu},doi={10.1/same}}\n@misc{m,title={Alpha}}",
        TidyOptions {
            merge: Some(MergeStrategy::Last),
            generate_keys: Some("[fulltitle:lower]".into()),
            sort: Some(vec!["key".into()]),
            ..TidyOptions::default()
        },
    ).unwrap();
    let doc = RawDocument::parse(&result.bibtex);
    assert_eq!(doc.entry_count(), 2);
    assert_eq!(doc.entry_keys(), ["alpha", "zulu"]);
    let id = doc.unique_entry("zulu").unwrap().unwrap();
    assert_eq!(doc.entry_info(id).unwrap().kind, "book");
    assert_eq!(result.count, 3);
    assert_eq!(
        result
            .renames
            .iter()
            .map(|rename| (
                rename.entry_id.index(),
                rename.old_key.as_str(),
                rename.new_key.as_str(),
            ))
            .collect::<Vec<_>>(),
        [(0, "z", "zulu"), (1, "a", "zulu"), (2, "m", "alpha")]
    );
}

#[test]
fn merging_connected_duplicate_groups_retains_each_field_once() {
    let result = tidy_bibtex(
        "@article{a,doi={10.1/a},left={A}}\n@book{b,doi={10.1/b},right={B}}\n@book{a,doi={10.1/b},bridge={C}}",
        TidyOptions {
            merge: Some(MergeStrategy::Combine),
            duplicates: Some(vec![DuplicateRule::Doi, DuplicateRule::Key]),
            ..TidyOptions::default()
        },
    ).unwrap();
    let doc = RawDocument::parse(&result.bibtex);
    assert_eq!(doc.entry_count(), 1);
    assert_eq!(doc.entry_keys(), ["a"]);
    let entry = doc.unique_entry("a").unwrap().unwrap();
    assert_eq!(
        doc.field_keys(entry).unwrap(),
        ["doi", "left", "right", "bridge"]
    );
}

#[test]
fn references_follow_generated_and_merged_keys_with_macro_expansion() {
    let result = tidy_bibtex(
        concat!(
            "@string{stem = {par}}\n",
            "@book{parent,title={Parent Book},doi={10.1/shared}}\n",
            "@book{alias,title={Parent Book},doi={10.1/shared}}\n",
            "@xdata{extra,title={Extra Data}}\n",
            "@inbook{child,title={Child},crossref=stem # {{ent}},xdata={{alias}, extra}}\n",
        ),
        TidyOptions {
            generate_keys: Some("[fulltitle:lower]".into()),
            merge: Some(MergeStrategy::First),
            ..TidyOptions::default()
        },
    )
    .unwrap();
    let doc = RawDocument::parse(&result.bibtex);
    assert_eq!(doc.entry_count(), 3);
    assert_eq!(doc.entry_keys(), ["parentbook", "extradata", "child"]);
    let entry = doc.unique_entry("child").unwrap().unwrap();
    let fields = doc.field_occurrences(entry).unwrap();
    assert_eq!(
        fields
            .iter()
            .find(|field| field.name == "crossref")
            .unwrap()
            .value,
        "parentbook"
    );
    assert_eq!(
        fields
            .iter()
            .find(|field| field.name == "xdata")
            .unwrap()
            .value,
        "parentbook, extradata"
    );
}

#[test]
fn renames_preserve_duplicate_occurrences_and_reject_ambiguous_references() {
    let source = "@book{same,title={Alpha}}\n@book{same,title={Beta}}";
    let options = TidyOptions {
        generate_keys: Some("[fulltitle:lower]".into()),
        ..TidyOptions::default()
    };
    let result = tidy_bibtex(source, options.clone()).unwrap();
    assert_eq!(
        result
            .renames
            .iter()
            .map(|rename| (
                rename.entry_id.index(),
                rename.old_key.as_str(),
                rename.new_key.as_str(),
            ))
            .collect::<Vec<_>>(),
        [(0, "same", "alpha"), (1, "same", "beta")]
    );
    let error = tidy_bibtex(
        &format!("{source}\n@inbook{{child,title={{Child}},crossref={{same}}}}"),
        options,
    )
    .unwrap_err();
    assert!(
        matches!(error, TidyError::Reference(message) if message.contains("ambiguous citation key"))
    );
}

#[test]
fn reference_keys_preserve_punctuation_under_text_formatting_options() {
    let result = tidy_bibtex(
        "@book{old,title={First},doi={10.1/same}}\n@book{A_B,title={Second},doi={10.1/same}}\n@inbook{child,title={Child},crossref={old}}",
        TidyOptions {
            merge: Some(MergeStrategy::Last),
            drop_all_caps: true,
            ..TidyOptions::default()
        },
    )
    .unwrap();
    let doc = RawDocument::parse(&result.bibtex);
    assert_eq!(doc.entry_keys(), ["A_B", "child"]);
    let child = doc.unique_entry("child").unwrap().unwrap();
    let crossref = doc.fields_for_key(child, "crossref").unwrap();
    assert_eq!(crossref[0].value, "A_B");
}

#[test]
fn renamed_crossrefs_preserve_normalized_inheritance() {
    let source = "@book{parent,title={Collected Work},year={2024}}\n@inbook{child,title={Chapter},crossref={{parent}}}";
    let result = tidy_bibtex(
        source,
        TidyOptions {
            generate_keys: Some("[fulltitle:lower]".into()),
            ..TidyOptions::default()
        },
    )
    .unwrap();
    let library =
        refkit_core::Library::parse_biblatex(&result.bibtex, refkit_core::RecoveryPolicy::Error)
            .unwrap();
    let chapter = library.get_record("chapter").unwrap();
    assert_eq!(chapter.title.as_deref(), Some("Chapter"));
    assert_eq!(chapter.date.as_deref(), Some("2024"));
}

#[test]
fn merges_reject_reference_cycles_in_the_retained_graph() {
    for source in [
        "@book{parent,title={P},doi={10.1/x}} @inbook{child,title={C},doi={10.1/x},crossref={parent}}",
        "@book{parent,title={P},doi={10.1/x}} @inbook{child,title={C},doi={10.1/x},xdata={parent}}",
        "@book{first,title={A},doi={10.1/x},crossref={middle}} @book{middle,title={B},crossref={last}} @book{last,title={C},doi={10.1/x}}",
    ] {
        refkit_core::Library::parse_biblatex(source, refkit_core::RecoveryPolicy::Error).unwrap();
        let strategy = if source.contains("first") {
            MergeStrategy::Combine
        } else {
            MergeStrategy::Last
        };
        let error = tidy_bibtex(
            source,
            TidyOptions {
                merge: Some(strategy),
                ..TidyOptions::default()
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, TidyError::Reference(message) if message.contains("reference cycle"))
        );
    }
}

#[test]
fn reference_normalization_preserves_builtin_macros_definitions_and_literals() {
    for (definitions, parent, reference) in [
        ("", "January", "jan"),
        ("@string{jan={jan}}", "jan", "jan"),
        ("", "jan", "{jan}"),
    ] {
        let source = format!(
            "{definitions}\n@book{{{parent},title={{Parent}},year={{2024}}}}\n@inbook{{c,title={{Child}},crossref={reference}}}"
        );
        refkit_core::Library::parse_biblatex(&source, refkit_core::RecoveryPolicy::Error).unwrap();
        let result = tidy_bibtex(
            &source,
            TidyOptions {
                generate_keys: Some("[fulltitle:lower]".into()),
                ..TidyOptions::default()
            },
        )
        .unwrap();
        let library = refkit_core::Library::parse_biblatex(
            &result.bibtex,
            refkit_core::RecoveryPolicy::Error,
        )
        .unwrap();
        assert_eq!(
            library.get_record("child").unwrap().date.as_deref(),
            Some("2024"),
            "source: {source}\noutput: {}",
            result.bibtex
        );
        let raw = RawDocument::parse(&result.bibtex);
        let child = raw.unique_entry("child").unwrap().unwrap();
        assert_eq!(
            raw.fields_for_key(child, "crossref").unwrap()[0].value,
            "parent"
        );
    }
}

#[test]
fn tidy_rejects_deep_values_before_reference_or_text_normalization() {
    let nested = format!("{}parent{}", "{".repeat(80), "}".repeat(80));
    for field in [
        format!("crossref={{{nested}}}"),
        format!("crossref=\"{nested}\""),
        format!("title={{{nested}}}"),
    ] {
        let source = format!("@book{{parent,title={{Parent}}}}\n@book{{child,{field}}}");
        let error = tidy_bibtex(
            &source,
            TidyOptions {
                generate_keys: Some("[fulltitle:lower]".into()),
                remove_braces: Some(vec!["title".into()]),
                ..TidyOptions::default()
            },
        )
        .unwrap_err();
        assert!(
            matches!(error, TidyError::Syntax { message, .. } if message.contains("nesting exceeds"))
        );
    }
}

#[test]
fn merged_reference_keys_preserve_typography_sensitive_identity() {
    let source = "@book{old,title={P},year={2024},doi={10.1/x}} @book{A--B,title={P},year={2024},doi={10.1/x}} @inbook{c,title={C},crossref={old}}";
    let original =
        refkit_core::Library::parse_biblatex(source, refkit_core::RecoveryPolicy::Error).unwrap();
    assert_eq!(
        original.get_record("c").unwrap().date.as_deref(),
        Some("2024")
    );
    let result = tidy_bibtex(
        source,
        TidyOptions {
            merge: Some(MergeStrategy::Last),
            ..TidyOptions::default()
        },
    )
    .unwrap();
    let library =
        refkit_core::Library::parse_biblatex(&result.bibtex, refkit_core::RecoveryPolicy::Error)
            .unwrap();
    assert!(library.contains_key("A--B"));
    assert_eq!(
        library.get_record("c").unwrap().date.as_deref(),
        Some("2024")
    );
}

#[test]
fn tidy_bounds_nested_optional_command_arguments() {
    let value = format!("{}text{}", "\\x[".repeat(80), "]".repeat(80));
    let error = tidy_bibtex(
        &format!("@misc{{a,title={{{value}}}}}"),
        TidyOptions {
            remove_braces: Some(vec!["title".into()]),
            ..TidyOptions::default()
        },
    )
    .unwrap_err();
    assert!(
        matches!(error, TidyError::Syntax { message, .. } if message.contains("nesting exceeds"))
    );
}

#[test]
fn reference_graph_follows_emitted_duplicate_and_omission_policy() {
    let source = "@book{a,title={Alpha},year={2024},crossref={b},crossref={outside}} @book{b,title={Beta},crossref={a}}";
    let original =
        refkit_core::Library::parse_biblatex(source, refkit_core::RecoveryPolicy::Error).unwrap();
    assert_eq!(
        original.get_record("b").unwrap().date.as_deref(),
        Some("2024")
    );
    for (omit, expected_date) in [(Vec::new(), Some("2024")), (vec!["crossref".into()], None)] {
        let result = tidy_bibtex(
            source,
            TidyOptions {
                generate_keys: Some("[fulltitle:lower]".into()),
                remove_duplicate_fields: false,
                omit,
                ..TidyOptions::default()
            },
        )
        .unwrap();
        let library = refkit_core::Library::parse_biblatex(
            &result.bibtex,
            refkit_core::RecoveryPolicy::Error,
        )
        .unwrap();
        assert_eq!(
            library.get_record("beta").unwrap().date.as_deref(),
            expected_date
        );
    }
    let error = tidy_bibtex(
        source,
        TidyOptions {
            generate_keys: Some("[fulltitle:lower]".into()),
            ..TidyOptions::default()
        },
    )
    .unwrap_err();
    assert!(matches!(error, TidyError::Reference(message) if message.contains("reference cycle")));
}

#[test]
fn escaped_delimiters_preserve_literal_text_during_brace_formatting() {
    let value = format!("{}text{}", "\\[".repeat(80), "]".repeat(80));
    let source = format!("@misc{{a,title={{{value}}}}}");
    let original =
        refkit_core::Library::parse_biblatex(&source, refkit_core::RecoveryPolicy::Error).unwrap();
    let result = tidy_bibtex(
        &source,
        TidyOptions {
            remove_braces: Some(vec!["title".into()]),
            ..TidyOptions::default()
        },
    )
    .unwrap();
    let library =
        refkit_core::Library::parse_biblatex(&result.bibtex, refkit_core::RecoveryPolicy::Error)
            .unwrap();
    assert_eq!(
        library.get_record("a").unwrap().title,
        original.get_record("a").unwrap().title
    );
}
