//! Recovery preserves independent records and original source coordinates at scale.
#![cfg(test)]

use std::fmt::Write as _;

use refkit_core::{DiagnosticAction, EntryField, Library, RecoveryPolicy};

#[test]
fn independent_unknown_values_have_no_recovery_pass_ceiling() {
    for count in [1, 129, 1000] {
        let mut source = "@string{unused=absent}\n@string{known={Known}}\n".to_string();
        for index in 0..count {
            writeln!(
                source,
                "@book{{item{index},title=未定{index} # {{ suffix}}}}"
            )
            .unwrap();
        }
        source.push_str("@book{good,title=known}");
        let library = Library::parse_biblatex(&source, RecoveryPolicy::Report).unwrap();
        assert_eq!(library.len(), count + 1);
        assert_eq!(library.diagnostics().len(), count);
        assert_eq!(
            library
                .get_record("good")
                .unwrap()
                .field(EntryField::Title)
                .as_deref(),
            Some("Known")
        );
        for (index, diagnostic) in library.diagnostics().iter().enumerate() {
            assert_eq!(diagnostic.action, DiagnosticAction::Literalized);
            assert_eq!(
                &source[diagnostic.span.clone().unwrap()],
                format!("未定{index}")
            );
        }
        assert!(Library::parse_biblatex(&source, RecoveryPolicy::Error).is_err());
    }
}

#[test]
fn unsafe_date_fields_recover_in_batches_without_dropping_records() {
    for count in [1, 129, 1000] {
        let mut source = String::new();
        for index in 0..count {
            writeln!(
                source,
                "@book{{item{index},title={{Retained}},year=2024,month=0}}"
            )
            .unwrap();
        }
        source.push_str("@book{good,title={Unaffected},year=2023}");
        let library = Library::parse_biblatex(&source, RecoveryPolicy::Report).unwrap();
        assert_eq!(library.len(), count + 1);
        assert_eq!(library.diagnostics().len(), count);
        for diagnostic in library.diagnostics() {
            assert_eq!(diagnostic.code, "invalid_field");
            assert_eq!(diagnostic.field.as_deref(), Some("month"));
            assert_eq!(diagnostic.action, DiagnosticAction::DroppedField);
            assert_eq!(&source[diagnostic.span.clone().unwrap()], "0");
        }
        assert_eq!(
            library
                .get_record("item0")
                .unwrap()
                .field(EntryField::Date)
                .as_deref(),
            Some("2024")
        );
        assert!(Library::parse_biblatex(&source, RecoveryPolicy::Error).is_err());
    }
}

#[test]
fn builtin_month_aliases_and_custom_definitions_keep_upstream_semantics() {
    for month in ["JAN", "jun", "Jul", "sep"] {
        let source = format!("@book{{a,title={{A}},year=2024,month={month}}}");
        let strict = Library::parse_biblatex(&source, RecoveryPolicy::Error).unwrap();
        let report = Library::parse_biblatex(&source, RecoveryPolicy::Report).unwrap();
        assert!(report.diagnostics().is_empty());
        assert_eq!(strict.to_json(), report.to_json());
    }
    let source = "@string{JAN={Custom}} @book{a,title=JAN}";
    let library = Library::parse_biblatex(source, RecoveryPolicy::Report).unwrap();
    assert_eq!(
        library
            .get_record("a")
            .unwrap()
            .field(EntryField::Title)
            .as_deref(),
        Some("Custom")
    );
    let source = "@book{a,title={A},year=2024,month=june}";
    assert!(Library::parse_biblatex(source, RecoveryPolicy::Error).is_err());
    let recovered = Library::parse_biblatex(source, RecoveryPolicy::Report).unwrap();
    assert_eq!(recovered.diagnostics().len(), 1);
    assert_eq!(recovered.diagnostics()[0].code, "unknown_abbreviation");
    assert_eq!(
        recovered.diagnostics()[0].action,
        DiagnosticAction::Literalized
    );
    assert_eq!(
        recovered
            .get_record("a")
            .unwrap()
            .field(EntryField::Date)
            .as_deref(),
        Some("2024-06")
    );
}

#[test]
fn date_guard_uses_the_last_duplicate_field_after_upstream_normalization() {
    let source = "@book{a,title={A},year=2024,month=0,MONTH=5}";
    for policy in [RecoveryPolicy::Error, RecoveryPolicy::Report] {
        let library = Library::parse_biblatex(source, policy).unwrap();
        assert!(library.diagnostics().is_empty());
        assert_eq!(
            library
                .get_record("a")
                .unwrap()
                .field(EntryField::Date)
                .as_deref(),
            Some("2024-05")
        );
    }
    let source = "@book{a,title={A},year=2024,month=5,MONTH=0}";
    assert!(Library::parse_biblatex(source, RecoveryPolicy::Error).is_err());
    let recovered = Library::parse_biblatex(source, RecoveryPolicy::Report).unwrap();
    assert_eq!(recovered.diagnostics().len(), 1);
    let diagnostic = &recovered.diagnostics()[0];
    assert_eq!(diagnostic.action, DiagnosticAction::DroppedField);
    assert_eq!(&source[diagnostic.span.clone().unwrap()], "0");
    assert_eq!(
        recovered
            .get_record("a")
            .unwrap()
            .field(EntryField::Date)
            .as_deref(),
        Some("2024-05")
    );
}

#[test]
fn date_guard_checks_normalized_nonbreaking_spaces_before_conversion() {
    let source = "@book{a,title={A},year=2024,month={Jan\u{a0} 9999}}\n@book{b,title={B}}";
    let strict = refkit_core::parse_bibtex_report(source, RecoveryPolicy::Error);
    assert!(!strict.ok);
    assert_eq!(strict.diagnostics[0].code, "invalid_field");
    let recovered = Library::parse_biblatex(source, RecoveryPolicy::Report).unwrap();
    assert_eq!(recovered.keys(), ["a", "b"]);
    assert_eq!(recovered.diagnostics().len(), 1);
    let diagnostic = &recovered.diagnostics()[0];
    assert_eq!(diagnostic.action, DiagnosticAction::DroppedField);
    assert_eq!(diagnostic.field.as_deref(), Some("month"));
    assert_eq!(
        &source[diagnostic.span.clone().unwrap()],
        "{Jan\u{a0} 9999}"
    );
    assert_eq!(
        recovered
            .get_record("a")
            .unwrap()
            .field(EntryField::Date)
            .as_deref(),
        Some("2024")
    );
}

#[test]
fn date_guard_rejects_pinned_parser_panic_inputs_without_losing_siblings() {
    for fields in [
        "year={- 2024}",
        "year={+ 2024}",
        "urlyear={- 2024}",
        "origyear={- 2024}",
        "eventyear={- 2024}",
        "date={²}",
        "month={²},year=2024",
        "month={Jan - 9999},year=2024",
        "month={123 9999},year=2024",
        "month={123Jan - 9999},year=2024",
        "date={2024-01-01T²:00:00}",
        "date={2024-00-XX}",
    ] {
        let source = format!("@book{{a,title={{A}},{fields}}}\n@book{{b,title={{B}}}}");
        let strict = refkit_core::parse_bibtex_report(&source, RecoveryPolicy::Error);
        assert!(!strict.ok, "{fields}");
        assert_eq!(strict.diagnostics[0].code, "invalid_field", "{fields}");
        let recovered = Library::parse_biblatex(&source, RecoveryPolicy::Report).unwrap();
        assert_eq!(recovered.keys(), ["a", "b"], "{fields}");
        assert_eq!(recovered.diagnostics().len(), 1, "{fields}");
        assert_eq!(
            recovered.diagnostics()[0].action,
            DiagnosticAction::DroppedField,
            "{fields}"
        );
    }
}

#[test]
fn month_guard_preserves_safe_cursor_consumption() {
    for value in ["Jan - 12", "123Jan - 12", "Jan9999", "123", "000"] {
        let source = format!("@book{{a,title={{A}},year=2024,month={{{value}}}}}");
        let library = Library::parse_biblatex(&source, RecoveryPolicy::Error).unwrap();
        assert!(library.diagnostics().is_empty(), "{value}");
        assert!(library.get_record("a").unwrap().date.is_some(), "{value}");
    }
}

#[test]
fn date_guard_preserves_non_numeric_literal_dates() {
    for value in [
        "Spring Ⅱ",
        "Spring²",
        "2024 Ⅱ",
        "2024 ²",
        "Ⅱ Spring",
        "2024 Spring²",
        "2024-02-31T²",
        "Spring/²",
        "2024?/²",
        "2024~/²",
        "2024%/²",
    ] {
        let source = format!("@book{{a,title={{A}},date={{{value}}}}}");
        let library = Library::parse_biblatex(&source, RecoveryPolicy::Error).unwrap();
        assert!(library.diagnostics().is_empty(), "{value}");
        let date = library.get_record("a").unwrap().date.as_ref().unwrap();
        assert!(
            matches!(&date.value, refkit_core::DateValue::Literal { text } if text == value),
            "{value}"
        );
    }
}

#[test]
fn inactive_years_with_overlong_digits_do_not_trigger_the_panic_guard() {
    for sign in ['+', '-'] {
        let source = format!("@book{{a,title={{A}},date={{2024}},year={{{sign} 12345}}}}");
        let library = Library::parse_biblatex(&source, RecoveryPolicy::Error).unwrap();
        assert!(library.diagnostics().is_empty());
        assert_eq!(
            library
                .get_record("a")
                .unwrap()
                .field(EntryField::Date)
                .as_deref(),
            Some("2024")
        );
    }
}

#[test]
fn reference_cycles_clear_only_the_effective_duplicate_field() {
    for name in ["crossref", "xdata"] {
        let source = format!(
            "@book{{a,{name}={{b}},{}={{a}}}}\n@book{{b,title={{Not inherited}}}}",
            name.to_ascii_uppercase()
        );
        let recovered = Library::parse_biblatex(&source, RecoveryPolicy::Report).unwrap();
        assert_eq!(recovered.keys(), ["a", "b"]);
        assert_eq!(recovered.diagnostics().len(), 1);
        let diagnostic = &recovered.diagnostics()[0];
        assert_eq!(diagnostic.code, "cyclic_reference");
        assert_eq!(diagnostic.action, DiagnosticAction::Literalized);
        assert_eq!(diagnostic.field.as_deref(), Some(name));
        assert_eq!(&source[diagnostic.span.clone().unwrap()], "{a}");
        assert!(recovered.get_record("a").unwrap().title.is_none());
        let raw = refkit_core::RawDocument::parse(&source);
        let entry = raw.entry_id_at(0).unwrap();
        let fields = raw.field_occurrences(entry).unwrap();
        assert_eq!(fields[0].value, "b");
        assert_eq!(fields[1].value, "a");
    }
}

#[test]
fn syntax_masking_and_value_recovery_share_original_utf8_coordinates() {
    let source = "@book{a,title=未定}\n@broken{unfinished\n@book{b,title=缺失}";
    let library = Library::parse_biblatex(source, RecoveryPolicy::Report).unwrap();
    assert_eq!(library.keys(), ["a", "b"]);
    for diagnostic in library.diagnostics() {
        let span = diagnostic.span.clone().unwrap();
        assert!(source.is_char_boundary(span.start));
        assert!(source.is_char_boundary(span.end));
        if diagnostic.code == "unknown_abbreviation" {
            assert!(["未定", "缺失"].contains(&&source[span]));
        }
    }
}

#[test]
fn independent_abbreviation_cycles_recover_in_one_batch() {
    for count in [1, 129, 1000] {
        let mut source = "@string{unused=unused}\n".to_string();
        for index in 0..count {
            writeln!(
                source,
                "@string{{cycle{index}=cycle{index}}}\n@book{{item{index},title=cycle{index}}}"
            )
            .unwrap();
        }
        source.push_str("@book{sentinel,title={Unchanged}}");
        let library = Library::parse_biblatex(&source, RecoveryPolicy::Report).unwrap();
        assert_eq!(library.len(), count + 1);
        assert_eq!(library.diagnostics().len(), count);
        for (index, diagnostic) in library.diagnostics().iter().enumerate() {
            assert_eq!(diagnostic.code, "cyclic_abbreviation");
            assert_eq!(diagnostic.action, DiagnosticAction::Literalized);
            assert_eq!(
                diagnostic.entry.as_deref(),
                Some(format!("item{index}").as_str())
            );
            assert_eq!(diagnostic.field.as_deref(), Some("title"));
            assert_eq!(
                &source[diagnostic.span.clone().unwrap()],
                format!("cycle{index}")
            );
        }
        assert!(Library::parse_biblatex(&source, RecoveryPolicy::Error).is_err());
    }
}

#[test]
fn independent_reference_cycles_recover_in_one_batch() {
    for count in [1, 129, 1000] {
        let mut source = String::new();
        for index in 0..count {
            writeln!(
                source,
                "@book{{item{index},title={{Retained}},crossref={{item{index}}}}}"
            )
            .unwrap();
        }
        source.push_str("@book{sentinel,title={Unchanged}}");
        let library = Library::parse_biblatex(&source, RecoveryPolicy::Report).unwrap();
        assert_eq!(library.len(), count + 1);
        assert_eq!(library.diagnostics().len(), count);
        for diagnostic in library.diagnostics() {
            assert_eq!(diagnostic.code, "cyclic_reference");
            assert_eq!(diagnostic.action, DiagnosticAction::Literalized);
            assert_eq!(diagnostic.field.as_deref(), Some("crossref"));
            assert_eq!(
                &source[diagnostic.span.clone().unwrap()],
                format!("{{{}}}", diagnostic.entry.as_deref().unwrap())
            );
        }
        assert!(Library::parse_biblatex(&source, RecoveryPolicy::Error).is_err());
    }
}

#[test]
fn shared_macro_cycle_uses_the_first_reachable_entry_field() {
    let source = "@string{one=two,two=one}\n@book{first,note=one}\n@book{second,title=one}";
    let library = Library::parse_biblatex(source, RecoveryPolicy::Report).unwrap();
    assert_eq!(library.len(), 2);
    assert_eq!(library.diagnostics().len(), 1);
    let diagnostic = &library.diagnostics()[0];
    assert_eq!(diagnostic.entry.as_deref(), Some("first"));
    assert_eq!(diagnostic.field.as_deref(), Some("note"));
    assert_eq!(&source[diagnostic.span.clone().unwrap()], "one");
}

#[test]
fn cascading_duplicate_field_recovery_has_a_cumulative_work_budget() {
    let source = format!(
        "@book{{a,title={{A}},year=2024,{}}}",
        "month=0,".repeat(1000)
    );
    let report = refkit_core::parse_bibtex_report(&source, RecoveryPolicy::Report);
    assert!(!report.ok);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "resource_limit")
    );
}

#[test]
fn malformed_latex_blocks_recover_within_the_rescan_budget() {
    let mut source = "@book{first,title={First}}\n".to_string();
    for index in 0..129 {
        writeln!(source, "@book{{bad{index},title={{$unclosed}}}}").unwrap();
    }
    source.push_str("@book{last,title={Last}}");
    let report = refkit_core::parse_bibtex_report(&source, RecoveryPolicy::Report);
    assert!(report.ok, "{:?}", report.diagnostics);
    assert_eq!(report.keys.unwrap(), ["first", "last"]);
    assert_eq!(report.diagnostics.len(), 129);
    assert!(
        report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.action == DiagnosticAction::DroppedBlock)
    );
}

#[test]
fn trailing_month_whitespace_is_a_field_local_failure() {
    for month in [
        "01 ",
        "January ",
        "01\u{a0}",
        "January\u{a0}",
        "01\u{2009}",
        "January\u{2009}",
    ] {
        let source = format!(
            "@misc{{a,title={{Retained}},month={{{month}}},year=2024}}\n@misc{{b,title={{Sentinel}}}}"
        );
        let strict = refkit_core::parse_bibtex_report(&source, RecoveryPolicy::Error);
        assert!(!strict.ok);
        assert_eq!(strict.diagnostics.len(), 1);
        assert_eq!(strict.diagnostics[0].field.as_deref(), Some("month"));
        assert_eq!(
            &source[strict.diagnostics[0].span.clone().unwrap()],
            format!("{{{month}}}")
        );
        let recovered = Library::parse_biblatex(&source, RecoveryPolicy::Report).unwrap();
        assert_eq!(recovered.keys(), ["a", "b"]);
        assert_eq!(
            recovered
                .get_record("a")
                .unwrap()
                .field(EntryField::Date)
                .as_deref(),
            Some("2024")
        );
        assert_eq!(recovered.diagnostics().len(), 1);
        let diagnostic = &recovered.diagnostics()[0];
        assert_eq!(diagnostic.action, DiagnosticAction::DroppedField);
        assert_eq!(diagnostic.field.as_deref(), Some("month"));
        assert_eq!(
            &source[diagnostic.span.clone().unwrap()],
            format!("{{{month}}}")
        );
    }
}

#[test]
fn month_error_attribution_respects_explicit_days_and_inactive_date_parts() {
    for (fields, date) in [
        ("year=2024,month={Jan },day=12", Some("2024-01-12")),
        (
            "date={2023-06-07},year=2024,month={January }",
            Some("2023-06-07"),
        ),
        ("month={January }", None),
    ] {
        let source = format!("@misc{{a,title={{Retained}},{fields}}}");
        for policy in [RecoveryPolicy::Error, RecoveryPolicy::Report] {
            let library = Library::parse_biblatex(&source, policy).unwrap();
            assert!(library.diagnostics().is_empty());
            assert_eq!(
                library
                    .get_record("a")
                    .unwrap()
                    .field(EntryField::Date)
                    .as_deref(),
                date
            );
        }
    }
    let source = "@misc{a,title={Retained},year={},month={January }}";
    let strict = refkit_core::parse_bibtex_report(source, RecoveryPolicy::Error);
    assert!(!strict.ok);
    assert_eq!(strict.diagnostics[0].field.as_deref(), Some("year"));
}

#[test]
fn month_recovery_uses_the_effective_raw_expression_span() {
    for fields in ["month=m # { }", "month={February },MONTH=m # { }"] {
        let source = format!("@book{{a,title={{Ä}},year=2024,{fields}}}\n@string{{m={{January}}}}");
        let expected_start = source.rfind("m # { }").unwrap();
        let expected = expected_start..expected_start + "m # { }".len();
        let strict = refkit_core::parse_bibtex_report(&source, RecoveryPolicy::Error);
        assert!(!strict.ok);
        assert_eq!(strict.diagnostics[0].span, Some(expected.clone()));
        let recovered = Library::parse_biblatex(&source, RecoveryPolicy::Report).unwrap();
        assert_eq!(recovered.keys(), ["a"]);
        assert_eq!(recovered.diagnostics().len(), 1);
        let diagnostic = &recovered.diagnostics()[0];
        assert_eq!(diagnostic.field.as_deref(), Some("month"));
        assert_eq!(diagnostic.action, DiagnosticAction::DroppedField);
        assert_eq!(diagnostic.span, Some(expected));
        assert_eq!(
            source.get(diagnostic.span.clone().unwrap()),
            Some("m # { }")
        );
    }
}

#[test]
fn inherited_month_recovery_omits_unowned_source_coordinates() {
    let source = "@xdata{child,title={Child},xdata={parent}}\n@xdata{parent,title={Parent},year=2024,month={Jan }}";
    let recovered = Library::parse_biblatex(source, RecoveryPolicy::Report).unwrap();
    assert_eq!(recovered.keys(), ["child", "parent"]);
    assert_eq!(recovered.diagnostics().len(), 2);
    for diagnostic in recovered.diagnostics() {
        assert_eq!(diagnostic.field.as_deref(), Some("month"));
        assert_eq!(diagnostic.action, DiagnosticAction::DroppedField);
        if diagnostic.entry.as_deref() == Some("child") {
            assert_eq!(diagnostic.span, None);
        } else {
            assert_eq!(diagnostic.entry.as_deref(), Some("parent"));
            assert_eq!(source.get(diagnostic.span.clone().unwrap()), Some("{Jan }"));
        }
    }
}

#[test]
fn invalid_scalar_date_parts_keep_forward_macro_expression_spans() {
    for field in ["year", "month"] {
        let source = format!("@misc{{a,title={{Ä}},{field}=m # {{ }}}}\n@string{{m={{invalid}}}}");
        let library = Library::parse_biblatex(&source, RecoveryPolicy::Report).unwrap();
        assert_eq!(library.keys(), ["a"]);
        assert_eq!(library.diagnostics().len(), 1);
        let diagnostic = &library.diagnostics()[0];
        assert_eq!(diagnostic.code, "invalid_field");
        assert_eq!(diagnostic.field.as_deref(), Some(field));
        assert_eq!(diagnostic.action, DiagnosticAction::DroppedField);
        assert_eq!(
            source.get(diagnostic.span.clone().unwrap()),
            Some("m # { }")
        );
    }
}
