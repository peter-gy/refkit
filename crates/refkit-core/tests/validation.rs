//! Inspect-only bibliographic quality and source-profile validation.

#![cfg(test)]

use refkit_core::{
    Date, DateParts, DateValue, EntryRecord, Library, RawDocument, Text, ValidationCode as Code,
    ValidationProfile,
};

fn record(key: &str, identifiers: &[(&str, &str)]) -> EntryRecord {
    EntryRecord {
        key: key.into(),
        entry_type: "Misc".into(),
        identifiers: identifiers
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect(),
        ..EntryRecord::default()
    }
}

#[test]
fn identifier_checks_are_inspect_only_and_never_imply_registration() {
    let library = Library::from_records(vec![
        record(
            "a",
            &[
                ("doi", "https://doi.org/10.1000/ABC"),
                ("isbn", "978-0-306-40615-7"),
                ("issn", "0317-8471"),
                ("orcid", "https://orcid.org/0000-0002-1694-233X"),
                ("custom", "anything"),
            ],
        ),
        record(
            "b",
            &[
                ("doi", "10.1000/abc"),
                ("isbn", "9780306406158"),
                ("issn", "0317-8472"),
                ("orcid", "0000-0002-1825-0098"),
            ],
        ),
    ])
    .unwrap();
    let before = library.to_json();
    let report = library.validate();
    assert_eq!(report.profile, ValidationProfile::Records);
    assert!(!report.is_valid());
    assert_eq!(
        report
            .issues
            .iter()
            .filter(|i| i.code == Code::InvalidIdentifier)
            .count(),
        3
    );
    let shared = report
        .issues
        .iter()
        .find(|i| i.code == Code::SharedIdentifier)
        .unwrap();
    assert_eq!(shared.target.entry, "a");
    assert_eq!(shared.related[0].entry, "b");
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.suggestion.as_deref() == Some("10.1000/abc"))
    );
    assert_eq!(library.to_json(), before);
}

#[test]
fn identifier_edge_cases_do_not_use_a_crossref_only_doi_regex() {
    let library = Library::from_records(vec![
        record(
            "valid",
            &[
                ("doi", "10.123.4/a/b;[c]"),
                ("isbn", "0-8044-2957-X"),
                ("orcid", "0000-0002-1825-0097"),
            ],
        ),
        record(
            "invalid",
            &[
                ("doi", "10../x"),
                ("isbn", "978030640615X"),
                ("issn", "X3178471"),
            ],
        ),
    ])
    .unwrap();
    let report = library.validate();
    assert_eq!(
        report
            .issues
            .iter()
            .filter(|i| i.code == Code::InvalidIdentifier)
            .count(),
        3
    );
    assert!(
        report
            .issues
            .iter()
            .filter(|i| i.code == Code::InvalidIdentifier)
            .all(|i| i.target.entry == "invalid")
    );
}

#[test]
fn record_checks_do_not_apply_biblatex_requirements() {
    let empty = Library::from_records(vec![record("minimal", &[])]).unwrap();
    assert!(empty.validate().issues.is_empty());
    let mut entry = record("a", &[]);
    entry.title = Some(Text::plain(" "));
    entry.url = Some(refkit_core::Url {
        value: "relative/file".into(),
        accessed: None,
    });
    entry.date = Some(Date {
        value: DateValue::Range {
            start: Some(DateParts {
                year: 2024,
                month: None,
                day: None,
                season: None,
                time: None,
            }),
            end: Some(DateParts {
                year: 2023,
                month: None,
                day: None,
                season: None,
                time: None,
            }),
        },
        uncertain: false,
        approximate: false,
    });
    entry.parents = vec![record("container", &[])];
    let library = Library::from_records(vec![entry]).unwrap();
    let codes: Vec<_> = library
        .validate()
        .issues
        .into_iter()
        .map(|i| i.code)
        .collect();
    assert_eq!(
        codes,
        [
            Code::EmptyTitle,
            Code::ReversedDateRange,
            Code::InvalidUrl,
            Code::IncompleteContainer
        ]
    );
}

#[test]
fn source_profile_uses_effective_fields_and_original_occurrence_targets() {
    let source = "% é retained\n@inproceedings{a,title={Paper},author={Doe, Jane},crossref={p},xdata={missing},doi={wrong}}\n@proceedings{p,title={Meeting},year=2024}";
    let document = RawDocument::parse(source);
    let report = document.validate().unwrap();
    assert_eq!(report.profile, ValidationProfile::Biblatex);
    let invalid = report
        .issues
        .iter()
        .find(|i| i.code == Code::InvalidIdentifier)
        .unwrap();
    assert_eq!(invalid.target.entry_id, Some(0));
    assert_eq!(invalid.target.field_id, Some(4));
    assert!(source[invalid.target.span.clone().unwrap()].contains("wrong"));
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == Code::UnresolvedReference && i.target.path == "xdata")
    );
    assert!(
        !report
            .issues
            .iter()
            .any(|i| i.code == Code::MissingRequiredField
                && i.target.entry == "a"
                && ["year", "booktitle"].contains(&i.target.path.as_str()))
    );
    assert_eq!(document.render().unwrap(), source);
}

#[test]
fn invalid_source_is_a_parse_failure_not_a_recovery_or_quality_report() {
    for source in [
        "@book{a, title={unclosed",
        "@misc{a, crossref={b}} @misc{b, crossref={a}}",
        "@misc{a,title=missingMacro}",
    ] {
        assert!(RawDocument::parse(source).validate().is_err());
    }
    let report =
        RawDocument::parse("@article{a,title={A},volume={nope},year=2024,month=2,day=99} ")
            .validate()
            .unwrap();
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == Code::MalformedField && i.target.path == "volume")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|i| i.code == Code::MalformedField && i.target.path == "date")
    );
}

#[test]
fn nested_container_identifiers_are_not_duplicate_top_level_works() {
    let parent = record("journal", &[("issn", "0317-8471")]);
    let mut a = record("a", &[]);
    let mut b = record("b", &[]);
    a.parents.push(parent.clone());
    b.parents.push(parent);
    assert!(
        Library::from_records(vec![a, b])
            .unwrap()
            .validate()
            .issues
            .is_empty()
    );
}

#[test]
fn malformed_date_inputs_fail_before_upstream_arithmetic() {
    for fields in [
        "date={123456X}",
        "date=danger",
        "year=2024,month=0",
        "year=2024,month={January 999}",
    ] {
        let source = format!(
            "@string{{danger={{123456X}}}} @misc{{bad,{fields}}} @misc{{good,title={{Good}}}}"
        );
        let failure = RawDocument::parse(&source).validate().unwrap_err();
        assert_eq!(failure.diagnostics[0].code, "invalid_field");
        assert!(Library::parse_biblatex(&source, refkit_core::RecoveryPolicy::Error).is_err());
        let recovered =
            Library::parse_biblatex(&source, refkit_core::RecoveryPolicy::Report).unwrap();
        assert_eq!(recovered.keys(), ["bad", "good"]);
        assert_eq!(
            recovered.diagnostics()[0].action,
            refkit_core::DiagnosticAction::DroppedField
        );
    }
    assert!(
        RawDocument::parse("@misc{a,date={\\textit{123456X}}}")
            .validate()
            .is_ok()
    );
}
