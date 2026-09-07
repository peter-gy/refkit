use refkit_core::{StyleError, prepare_style_from_xml};

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
            let error = prepare_style_from_xml(&source).unwrap_err();
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
            let result = prepare_style_from_xml(&custom_style("", &citation));
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

    let style = prepare_style_from_xml(&custom_style("", r#"<choose><if locator="sub-verbo"><text value="dictionary entry"/></if><else><text value="work"/></else></choose>"#)).unwrap();
    let library =
        Library::parse_biblatex("@book{a,title={Dictionary}}", RecoveryPolicy::Error).unwrap();
    let document = Document::new(Arc::new(library), Arc::new(style), None);
    let result = document
        .render(vec![CitationRequest::new(
            vec![Cite::new(
                "a".to_string(),
                Some("entry".to_string()),
                Some("sub verbo".to_string()),
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
