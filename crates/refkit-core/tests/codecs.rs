//! Interchange roundtrips, explicit loss reporting, and malformed input.

#![cfg(test)]

use refkit_core::{
    BibliographyFormat as Format, DateValue, LossPolicy, RecoveryPolicy, convert, decode, encode,
};

const BOOK: &str = r#"[{"id":"book","type":"book","title":"A Book","author":[{"family":"Doe","given":"Jane"}],"issued":{"date-parts":[[2024]]},"publisher":"Press","language":"en-US","DOI":"10.1234/book"}]"#;

#[test]
fn source_type_annotations_cannot_turn_records_into_comment_blocks() {
    let library = refkit_core::Library::from_json(
        r#"{"schema_version":1,"records":[{"key":"a","entry_type":"Book","extensions":{"biblatex":{"@type":"comment"}}}]}"#,
    ).unwrap();
    for format in [Format::Biblatex, Format::CslJson] {
        let encoded = encode(&library, format, LossPolicy::Error).unwrap();
        let restored = decode(
            &encoded.text,
            format,
            LossPolicy::Error,
            RecoveryPolicy::Error,
        )
        .unwrap();
        assert_eq!(restored.library.get_record("a").unwrap().entry_type, "Book");
    }
}

#[test]
fn unchecked_raw_dates_are_retained_as_literals_or_refused_by_loss_policy() {
    let source = r#"[{"id":"a","type":"book","issued":{"raw":"123456X"}}]"#;
    let report = decode(
        source,
        Format::CslJson,
        LossPolicy::Report,
        RecoveryPolicy::Error,
    )
    .unwrap();
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.code == "date_literalized")
    );
    assert!(
        matches!(&report.library.get_record("a").unwrap().date.as_ref().unwrap().value,
        DateValue::Literal { text } if text == "123456X")
    );
    assert!(
        decode(
            source,
            Format::CslJson,
            LossPolicy::Error,
            RecoveryPolicy::Error
        )
        .is_err()
    );
}

#[test]
fn malformed_date_annotations_do_not_override_structured_dates() {
    let library = refkit_core::Library::from_json(
        r#"{"schema_version":1,"records":[{"key":"a","entry_type":"Book","date":{"value":{"kind":"point","date":{"year":2024}}},"extensions":{"biblatex":{"@date":"123456X"}}}]}"#,
    ).unwrap();
    let report = encode(&library, Format::Biblatex, LossPolicy::Error).unwrap();
    let restored = decode(
        &report.text,
        Format::Biblatex,
        LossPolicy::Error,
        RecoveryPolicy::Error,
    )
    .unwrap();
    assert_eq!(
        restored.library.get_record("a").unwrap().date,
        library.get_record("a").unwrap().date
    );
}

#[test]
fn common_book_fields_roundtrip_through_every_codec() {
    let input = decode(
        BOOK,
        Format::CslJson,
        LossPolicy::Error,
        RecoveryPolicy::Error,
    )
    .unwrap();
    for format in [Format::Biblatex, Format::Hayagriva, Format::CslJson] {
        let output = encode(&input.library, format, LossPolicy::Error)
            .unwrap_or_else(|error| panic!("{format:?}: {error:?}"));
        assert!(output.issues.iter().all(|issue| !issue.lossy));
        let restored = decode(
            &output.text,
            format,
            LossPolicy::Error,
            RecoveryPolicy::Error,
        )
        .unwrap();
        assert_eq!(restored.library.keys(), &["book"]);
        assert_eq!(
            restored.library.records()[0]
                .title
                .as_ref()
                .unwrap()
                .plain_text(),
            "A Book"
        );
        assert_eq!(
            restored.library.records()[0].identifiers["doi"],
            "10.1234/book"
        );
    }
}

#[test]
fn article_containers_and_numeric_fields_convert_without_loss() {
    let source = "@article{paper,author={Doe, Jane},title={Study},date={2024-02-29},journal={Journal},volume={12},number={3},pages={10-20},publisher={Press},doi={10.1234/paper}}";
    let output = convert(
        source,
        Format::Biblatex,
        Format::CslJson,
        LossPolicy::Error,
        RecoveryPolicy::Error,
    )
    .unwrap_or_else(|error| panic!("{error:?}"));
    let decoded = decode(
        &output.text,
        Format::CslJson,
        LossPolicy::Error,
        RecoveryPolicy::Error,
    )
    .unwrap();
    assert_eq!(
        decoded.library.records()[0].parents[0]
            .title
            .as_ref()
            .unwrap()
            .plain_text(),
        "Journal"
    );
    assert_eq!(
        decoded.library.records()[0].parents[0]
            .volume
            .as_ref()
            .unwrap()
            .value(),
        "12"
    );
}

#[test]
fn csl_date_ranges_are_retained_and_yaml_loss_is_explicit() {
    let source = r#"[{"id":"range","type":"book","issued":{"date-parts":[[2020,1],[2024,2]],"circa":true}}]"#;
    let input = decode(
        source,
        Format::CslJson,
        LossPolicy::Error,
        RecoveryPolicy::Error,
    )
    .unwrap();
    assert!(matches!(
        input.library.records()[0].date.as_ref().unwrap().value,
        DateValue::Range { .. }
    ));
    encode(&input.library, Format::CslJson, LossPolicy::Error).unwrap();
    let before = input.library.to_json();
    let error = encode(&input.library, Format::Hayagriva, LossPolicy::Error).unwrap_err();
    assert!(
        error
            .issues
            .iter()
            .any(|issue| issue.path.starts_with("date") && issue.lossy)
    );
    let output = encode(&input.library, Format::Hayagriva, LossPolicy::Report).unwrap();
    assert!(output.issues.iter().any(|issue| issue.lossy));
    assert_eq!(input.library.to_json(), before);
}

#[test]
fn source_extensions_survive_their_format_and_report_cross_format_loss() {
    let source = r#"[{"id":"custom","type":"book","custom":{"rank":3}}]"#;
    let input = decode(
        source,
        Format::CslJson,
        LossPolicy::Error,
        RecoveryPolicy::Error,
    )
    .unwrap();
    assert_eq!(input.issues[0].code, "retained_extension");
    encode(&input.library, Format::CslJson, LossPolicy::Error).unwrap();
    let output = encode(&input.library, Format::Biblatex, LossPolicy::Report).unwrap();
    assert!(
        output
            .issues
            .iter()
            .any(|issue| issue.path == "extensions.csl-json.custom.rank" && issue.lossy)
    );
    let bib = decode(
        "@book{a,title={A {Protected} Title},custom={Keep {THIS}}}",
        Format::Biblatex,
        LossPolicy::Error,
        RecoveryPolicy::Error,
    )
    .unwrap();
    encode(&bib.library, Format::Biblatex, LossPolicy::Error)
        .unwrap_or_else(|error| panic!("{error:?}"));
    assert!(encode(&bib.library, Format::CslJson, LossPolicy::Error).is_err());
}

#[test]
fn malformed_sources_and_duplicate_ids_fail_before_output() {
    for source in [
        r#"[{"id":"a","type":"book"},{"id":"a","type":"book"}]"#,
        r#"[{"id":"a","id":"b","type":"book"}]"#,
        r#"[{"id":"a","type":"book","issued":{"date-parts":[[2024,99]]}}]"#,
    ] {
        assert!(
            decode(
                source,
                Format::CslJson,
                LossPolicy::Report,
                RecoveryPolicy::Error
            )
            .is_err()
        );
    }
    let error = decode(
        "@book{",
        Format::Biblatex,
        LossPolicy::Report,
        RecoveryPolicy::Error,
    )
    .err()
    .unwrap();
    assert!(!error.diagnostics.is_empty());
}

#[test]
fn numeric_narrowing_and_type_specialization_are_reported() {
    let large = decode(
        "@book{a,volume={2147483648}}",
        Format::Biblatex,
        LossPolicy::Report,
        RecoveryPolicy::Error,
    )
    .unwrap();
    assert!(
        large
            .issues
            .iter()
            .any(|issue| issue.code == "numeric_literalized" && issue.path == "volume")
    );
    assert_eq!(
        large.library.records()[0].volume.as_ref().unwrap().value(),
        "2147483648"
    );
    assert!(
        decode(
            "@book{a,volume={2147483648}}",
            Format::Biblatex,
            LossPolicy::Error,
            RecoveryPolicy::Error
        )
        .is_err()
    );
    let dataset = decode(
        r#"[{"id":"data","type":"dataset","title":"Data"}]"#,
        Format::CslJson,
        LossPolicy::Error,
        RecoveryPolicy::Error,
    )
    .unwrap();
    let bib = encode(&dataset.library, Format::Biblatex, LossPolicy::Error).unwrap();
    let roundtrip = convert(
        &bib.text,
        Format::Biblatex,
        Format::CslJson,
        LossPolicy::Error,
        RecoveryPolicy::Error,
    )
    .unwrap();
    assert!(roundtrip.text.contains("\"dataset\""));
    assert!(encode(&dataset.library, Format::Hayagriva, LossPolicy::Error).is_err());
}

#[test]
fn ignored_engine_fields_are_preserved_as_extensions() {
    let source = "@book{a,title={A},subtitle={Subtitle},langid={chinese}}";
    let input = decode(
        source,
        Format::Biblatex,
        LossPolicy::Error,
        RecoveryPolicy::Error,
    )
    .unwrap();
    let output = encode(&input.library, Format::Biblatex, LossPolicy::Error).unwrap();
    assert!(output.text.contains("subtitle"));
    assert!(output.text.contains("chinese"));
    let changed = encode(&input.library, Format::CslJson, LossPolicy::Report).unwrap();
    assert!(
        changed
            .issues
            .iter()
            .any(|issue| issue.path.starts_with("extensions.biblatex.subtitle") && issue.lossy)
    );
}

#[test]
fn unmodeled_csl_types_roundtrip_but_cannot_silently_become_yaml_misc() {
    let input = decode(
        r#"[{"id":"a","type":"hearing"}]"#,
        Format::CslJson,
        LossPolicy::Error,
        RecoveryPolicy::Error,
    )
    .unwrap();
    assert_eq!(input.issues[0].code, "type_projected");
    let output = encode(&input.library, Format::CslJson, LossPolicy::Error).unwrap();
    assert!(output.text.contains("hearing"));
    assert!(encode(&input.library, Format::Hayagriva, LossPolicy::Error).is_err());
    assert!(encode(&input.library, Format::Biblatex, LossPolicy::Error).is_err());
}

#[test]
fn rich_bibliographies_produce_decodable_output_or_explicit_loss() {
    for (source, source_format) in [
        (
            include_str!("../../../packages/refkit/tests/fixtures/hayagriva-rich.yaml"),
            Format::Hayagriva,
        ),
        (
            include_str!("../../../packages/refkit/tests/fixtures/typst-biblatex.bib"),
            Format::Biblatex,
        ),
    ] {
        let input = decode(
            source,
            source_format,
            LossPolicy::Report,
            RecoveryPolicy::Error,
        )
        .unwrap();
        for format in [Format::Biblatex, Format::Hayagriva, Format::CslJson] {
            let output = encode(&input.library, format, LossPolicy::Report)
                .unwrap_or_else(|error| panic!("{source_format:?} -> {format:?}: {error:?}"));
            let restored = decode(
                &output.text,
                format,
                LossPolicy::Report,
                RecoveryPolicy::Error,
            )
            .unwrap();
            assert_eq!(restored.library.keys(), input.library.keys());
            if output.issues.iter().any(|issue| issue.lossy) {
                assert!(encode(&input.library, format, LossPolicy::Error).is_err());
            }
        }
    }
}

#[test]
fn csl_dates_and_archive_fields_reach_rendering() {
    let source = r#"[{"id":"a","type":"book","issued":{"date-parts":[[2024]]},"event-date":{"date-parts":[[2023]]},"original-date":{"date-parts":[[1900]]},"archive_location":"Shelf 4","call-number":"AB123"}]"#;
    let decoded = decode(
        source,
        Format::CslJson,
        LossPolicy::Error,
        RecoveryPolicy::Error,
    )
    .unwrap();
    let style = refkit_core::prepare_style_from_xml(r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text"><info><title>Dates</title><id>https://example.test/dates</id></info><citation><layout delimiter=";"><date variable="issued"><date-part name="year"/></date><date variable="event-date"><date-part name="year"/></date><date variable="original-date"><date-part name="year"/></date><text variable="archive_location"/><text variable="call-number"/></layout></citation></style>"#, None).unwrap();
    let rendered =
        refkit_core::render_library_citation(&decoded.library, "a", &style, None).unwrap();
    for value in ["2024", "2023", "1900", "Shelf 4", "AB123"] {
        assert!(
            rendered.text.contains(value),
            "missing {value}: {}",
            rendered.text
        );
    }
    encode(&decoded.library, Format::CslJson, LossPolicy::Error).unwrap();
}
