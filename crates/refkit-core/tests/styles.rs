use refkit_core::{StyleError, prepare_style_from_xml};

const CHILD_STYLE: &str = r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" default-locale="de-DE">
<info><title>Child</title><id>https://example.com/child</id>
<link rel="independent-parent" href="https://example.com/parent"/></info></style>"#;
const PARENT_STYLE: &str = r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text" default-locale="en-US">
<info><title>Parent</title><id>https://example.com/parent</id></info>
<locale xml:lang="de-DE"><terms><term name="page">Seite</term></terms></locale>
<locale xml:lang="en-US"><terms><term name="page">page</term></terms></locale>
<citation><layout><text term="page"/></layout></citation></style>"#;

#[test]
fn supplied_parent_preserves_child_identity_and_locale_precedence() {
    use refkit_core::{Library, RecoveryPolicy, render_library_citation};
    let style = prepare_style_from_xml(CHILD_STYLE, Some(PARENT_STYLE)).unwrap();
    assert_eq!(style.title(), "Child");
    assert_eq!(style.csl_id(), "https://example.com/child");
    let library = Library::parse_biblatex("@book{a,title={A}}", RecoveryPolicy::Error).unwrap();
    assert_eq!(
        render_library_citation(&library, "a", &style, None)
            .unwrap()
            .text,
        "Seite"
    );
    assert_eq!(
        render_library_citation(&library, "a", &style, Some("en-US"))
            .unwrap()
            .text,
        "page"
    );
}

#[test]
fn supplied_parent_requires_matching_independent_identity_and_valid_xml() {
    assert!(matches!(
        prepare_style_from_xml(CHILD_STYLE, None),
        Err(StyleError::MissingParent(_))
    ));
    assert!(matches!(
        prepare_style_from_xml(CHILD_STYLE, Some(CHILD_STYLE)),
        Err(StyleError::InvalidParent(_))
    ));
    assert!(matches!(
        prepare_style_from_xml(PARENT_STYLE, Some(PARENT_STYLE)),
        Err(StyleError::InvalidParent(_))
    ));
    let wrong = PARENT_STYLE.replace("https://example.com/parent", "https://example.com/wrong");
    assert!(matches!(
        prepare_style_from_xml(CHILD_STYLE, Some(&wrong)),
        Err(StyleError::InvalidParent(_))
    ));
    let missing_macro = PARENT_STYLE.replace("<text term=\"page\"/>", "<text macro=\"missing\"/>");
    assert!(matches!(
        prepare_style_from_xml(CHILD_STYLE, Some(&missing_macro)),
        Err(StyleError::InvalidMacro(_))
    ));
    let oversized = format!("{PARENT_STYLE}<!--{}-->", "x".repeat(2_097_152));
    assert!(matches!(
        prepare_style_from_xml(CHILD_STYLE, Some(&oversized)),
        Err(StyleError::InvalidXml(_))
    ));
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn custom_style_checks_xml_limits_in_subprocess() {
    const CHILD: &str = "REFKIT_STYLE_PREFLIGHT_CHILD";
    if std::env::var_os(CHILD).is_some() {
        let deep = custom_style("", &nested_groups(10_000, r#"<text value="end"/>"#));
        let wide = custom_style(
            &"<!-- comment -->".repeat(100_001),
            r#"<text value="end"/>"#,
        );
        let oversized = custom_style(
            "",
            &format!("<!--{}--><text value=\"end\"/>", "x".repeat(2_097_152)),
        );
        for (source, expected) in [
            (deep, "64 elements"),
            (wide, "100000 XML nodes"),
            (oversized, "2097152 bytes"),
        ] {
            let error = prepare_style_from_xml(&source, None).unwrap_err();
            assert!(matches!(error, StyleError::InvalidXml(detail) if detail.contains(expected)));
        }
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "custom_style_checks_xml_limits_in_subprocess",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn custom_style_limits_attributes_on_start_and_empty_elements() {
    for empty in [false, true] {
        for count in [256, 257] {
            let namespaces = (0..count - usize::from(empty))
                .map(|index| format!(r#" xmlns:p{index}="urn:refkit:{index}""#))
                .collect::<String>();
            let citation = if empty {
                format!(r#"<text value="bounded"{namespaces}/>"#)
            } else {
                format!(r#"<group{namespaces}><text value="bounded"/></group>"#)
            };
            let result = prepare_style_from_xml(&custom_style("", &citation), None);
            if count == 256 {
                assert_eq!(result.unwrap().title(), "Macro contract");
            } else {
                assert!(
                    matches!(result, Err(StyleError::InvalidXml(detail)) if detail == "element exceeds 256 attributes")
                );
            }
        }
    }
}

#[test]
fn custom_style_matches_sub_verbo_locator_conditions() {
    use refkit_core::{CitationRequest, Cite, Document, Library, RecoveryPolicy};
    use std::sync::Arc;

    let style = prepare_style_from_xml(&custom_style("", r#"<choose><if locator="sub-verbo"><text value="dictionary entry"/></if><else><text value="work"/></else></choose>"#), None).unwrap();
    let library =
        Library::parse_biblatex("@book{a,title={Dictionary}}", RecoveryPolicy::Error).unwrap();
    let document = Document::new(Arc::new(library), Arc::new(style), None);
    let result = document
        .render(vec![CitationRequest::new(
            vec![Cite::new(
                "a".to_string(),
                Some("entry".to_string()),
                Some("sub verbo".to_string()),
                refkit_core::CitePurpose::Normal,
            )],
            None,
        )])
        .unwrap();
    assert_eq!(result.citations[0].text, "dictionary entry");
}

fn nested_groups(depth: usize, content: &str) -> String {
    format!(
        "{}{content}{}",
        "<group>".repeat(depth),
        "</group>".repeat(depth)
    )
}

fn custom_style(macros: &str, citation: &str) -> String {
    format!(
        r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
    <info><title>Macro contract</title><id>https://example.com/macros</id><updated>2026-01-01T00:00:00Z</updated></info>
    {macros}<citation><layout>{citation}</layout></citation></style>"#
    )
}
