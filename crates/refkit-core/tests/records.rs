use std::collections::BTreeMap;

use refkit_core::{
    DateValue, EntryRecord, ExtensionValue, Library, Name, RecoveryPolicy, Text, TextKind,
    load_prepared_style, render_library_bibliography,
};

#[test]
fn records_reconstruct_protected_text_names_dates_containers_and_rendering() {
    let source = r"@article{paper,
author={family=Doe, given=Jane, given-i=J., id=researcher},
title={Study of {DNA} and $x$}, shorttitle={DNA},
date={2020-02-29/2022-03-01?}, journal={Research Journal}, volume={12},
pages={10-20}, doi={10.1234/study}, keywords={biology, methods},
custom={Keep {THIS}}
}";
    let library = Library::parse_biblatex(source, RecoveryPolicy::Error).unwrap();
    let record = &library.records()[0];
    assert!(
        record
            .title
            .as_ref()
            .unwrap()
            .chunks
            .iter()
            .any(|chunk| chunk.kind == TextKind::Protected && chunk.text == "DNA")
    );
    assert!(
        record
            .title
            .as_ref()
            .unwrap()
            .chunks
            .iter()
            .any(|chunk| chunk.kind == TextKind::Math && chunk.text == "x")
    );
    assert!(record.title.as_ref().unwrap().short.is_some());
    assert!(
        matches!(&record.authors[0], Name::Person { id: Some(id), given_initials: Some(initials), .. } if id == "researcher" && initials == "J.")
    );
    let date = record.date.as_ref().unwrap();
    assert!(date.uncertain);
    assert!(!date.approximate);
    assert!(
        matches!(&date.value, DateValue::Range { start: Some(start), end: Some(end) } if start.year == 2020 && start.month == Some(2) && start.day == Some(29) && end.year == 2022)
    );
    assert_eq!(
        record.parents[0].title.as_ref().unwrap().plain_text(),
        "Research Journal"
    );
    assert!(record.extensions["biblatex"].contains_key("custom"));
    let reconstructed = Library::from_records(library.records().to_vec()).unwrap();
    let decoded = Library::from_json(&library.to_json()).unwrap();
    assert_eq!(library.records(), reconstructed.records());
    assert_eq!(library.records(), decoded.records());
    let style = load_prepared_style("apa").unwrap();
    let original = render_library_bibliography(&library, &style, Some("en-US")).unwrap();
    let roundtrip = render_library_bibliography(&decoded, &style, Some("en-US")).unwrap();
    assert_eq!(original.text, roundtrip.text);
    assert_eq!(original.html, roundtrip.html);
}

#[test]
fn structured_construction_is_direct_and_rejects_duplicate_keys() {
    let record = EntryRecord {
        key: "org".into(),
        entry_type: "Book".into(),
        title: Some(Text::plain("Annual Report")),
        authors: vec![Name::Organization {
            name: "Research Council".into(),
        }],
        ..EntryRecord::default()
    };
    let library = Library::from_records(vec![record.clone()]).unwrap();
    assert_eq!(library.records()[0], record);
    assert!(
        Library::from_records(vec![record.clone(), record])
            .err()
            .unwrap()
            .to_string()
            .contains("duplicate entry key")
    );
}

#[test]
fn yaml_extensions_and_nested_containers_survive_record_snapshots() {
    let library = Library::parse_hayagriva_yaml("a:\n  type: Article\n  custom: {reviewed: true}\n  parent:\n    type: Periodical\n    custom-parent: retained\n").unwrap();
    let record = &library.records()[0];
    assert!(record.extensions["hayagriva"].contains_key("custom"));
    assert!(record.parents[0].extensions["hayagriva"].contains_key("custom-parent"));
    assert_eq!(
        Library::from_json(&library.to_json()).unwrap().records(),
        library.records()
    );
}

#[test]
fn mixed_creators_preserve_literal_organizations_and_editor_roles() {
    let library = Library::parse_biblatex("@book{a,title={A},author={{Research Council} and Doe, Jane},editor={Roe, Richard},editortype={translator}}", RecoveryPolicy::Error).unwrap();
    let record = &library.records()[0];
    assert!(
        matches!(&record.authors[0], Name::Organization { name } if name == "Research Council")
    );
    assert!(matches!(&record.authors[1], Name::Person { family, .. } if family == "Doe"));
    assert!(record.editors.is_empty());
    assert_eq!(record.affiliated[0].role, "translator");
}

#[test]
fn deepest_supported_container_chain_roundtrips_through_snapshot_json() {
    let mut record = EntryRecord {
        key: "a".into(),
        entry_type: "Book".into(),
        ..EntryRecord::default()
    };
    for _ in 0..64 {
        record = EntryRecord {
            key: "a".into(),
            entry_type: "Book".into(),
            parents: vec![record],
            ..EntryRecord::default()
        };
    }
    let library = Library::from_records(vec![record]).unwrap();
    assert_eq!(
        Library::from_json(&library.to_json()).unwrap().records(),
        library.records()
    );
    let oversized = format!("{}0{}", "[".repeat(257), "]".repeat(257));
    assert!(refkit_core::validate_record_source(&oversized).is_err());
}

#[test]
fn records_retain_urls_that_the_renderer_cannot_prepare() {
    let library =
        Library::parse_biblatex("@book{a,title={A},url={not a URL}}", RecoveryPolicy::Error)
            .unwrap();
    assert_eq!(
        library.records()[0].url.as_ref().unwrap().value,
        "not a URL"
    );
    assert_eq!(
        Library::from_json(&library.to_json()).unwrap().records(),
        library.records()
    );
}

#[test]
fn signed_calendar_years_preserve_complete_times() {
    let library = Library::from_json(r#"{"schema_version":1,"records":[{"key":"a","entry_type":"Book","date":{"value":{"kind":"point","date":{"year":-1,"month":1,"day":2,"time":"03:04:05Z"}}}}]}"#).unwrap();
    assert_eq!(
        library.records()[0].date.as_ref().unwrap().display(),
        "-0001-01-02T03:04:05Z"
    );
    assert_eq!(
        Library::from_json(&library.to_json()).unwrap().records(),
        library.records()
    );
}

#[test]
fn record_snapshots_validate_version_shapes_and_extension_numbers() {
    for source in [
        r#"{"schema_version":2,"records":[]}"#,
        r#"{"schema_version":1,"records":[],"records":[]}"#,
        r#"{"schema_version":1,"records":[{"key":"a","entry_type":"Book","unknown":true}]}"#,
        r#"{"schema_version":1,"records":[{"key":"a","entry_type":"Book","date":{"value":{"kind":"point","date":{"year":2023,"month":2,"day":29}}}}]}"#,
    ] {
        assert!(Library::from_json(source).is_err());
    }
    let record = EntryRecord {
        key: "a".into(),
        entry_type: "Book".into(),
        extensions: BTreeMap::from([(
            "app".into(),
            BTreeMap::from([("score".into(), ExtensionValue::Number(f64::NAN))]),
        )]),
        ..EntryRecord::default()
    };
    assert!(Library::from_records(vec![record]).is_err());
}
