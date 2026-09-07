use std::sync::Arc;

use refkit_core::{
    BibliographyLayout, CitationRequest, Cite, Document, DocumentError, FontStyle, Library,
    PreparedStyle, RecoveryPolicy, RenderedMeta, RenderedNode, prepare_style_from_xml,
    render_library_citation, render_library_citation_each, render_library_citation_group,
};

fn style(citation: &str, bibliography: &str, class: &str) -> Arc<PreparedStyle> {
    Arc::new(prepare_style_from_xml(&format!(
        r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="{class}">
        <info><title>Render contract</title><id>https://example.com/render</id><updated>2026-01-01T00:00:00Z</updated></info>
        {citation}{bibliography}</style>"#,
    )).unwrap())
}

fn library() -> Arc<Library> {
    Arc::new(Library::parse_biblatex(
        "@book{a, author={Alpha, Ann}, title={Alpha}, year={2026}}\n@book{b, author={Beta, Ben}, title={Beta}, year={2026}}",
        RecoveryPolicy::Error,
    ).unwrap())
}

fn request(keys: &[&str], note_number: Option<usize>) -> CitationRequest {
    CitationRequest::new(
        keys.iter()
            .map(|key| Cite::new((*key).to_string(), None, None))
            .collect(),
        note_number,
    )
}

#[test]
fn scalar_sequence_and_group_preserve_their_numbering_scope() {
    let library = library();
    let style = style(
        r#"<citation><layout prefix="[" suffix="]" delimiter=", "><text variable="citation-number"/></layout></citation>"#,
        r#"<bibliography><sort><key variable="title"/></sort><layout><text variable="title"/></layout></bibliography>"#,
        "in-text",
    );
    assert_eq!(
        render_library_citation(&library, "b", &style, None)
            .unwrap()
            .text,
        "[1]"
    );
    let each = render_library_citation_each(&library, &["b", "a"], &style, None).unwrap();
    assert_eq!(
        each.iter()
            .map(|item| item.text.as_str())
            .collect::<Vec<_>>(),
        ["[2]", "[1]"]
    );
    let group = render_library_citation_group(&library, &["b", "a"], &style, None).unwrap();
    assert_eq!(group.text, "[2, 1]");
    let document = Document::new(library, style, None);
    assert_eq!(
        document
            .render(vec![request(&["b", "a"], None)])
            .unwrap()
            .citations[0]
            .output(),
        group
    );
}

#[test]
fn sorted_citation_nodes_identify_original_request_items() {
    let style = style(
        r#"<citation><sort><key variable="title"/></sort><layout delimiter=", "><text variable="title"/></layout></citation>"#,
        "",
        "in-text",
    );
    let rendered = Document::new(library(), style, None)
        .render(vec![request(&["b", "a"], None)])
        .unwrap();
    let mut identities = Vec::new();
    fn visit(nodes: &[RenderedNode], result: &mut Vec<(String, usize)>) {
        for node in nodes {
            if let RenderedNode::Element { meta, children, .. } = node {
                if let Some(RenderedMeta::Entry { key, item_index }) = meta {
                    result.push((key.clone(), *item_index));
                }
                visit(children, result);
            }
        }
    }
    visit(rendered.citations[0].tree_nodes(), &mut identities);
    assert_eq!(identities, [("a".to_string(), 1), ("b".to_string(), 0)]);
}

#[test]
fn note_context_is_local_to_one_render_call() {
    let style = style(
        r#"<citation><layout delimiter=", "><number variable="first-reference-note-number"/><choose><if position="near-note"><text value=" near"/></if><else><text value=" far"/></else></choose></layout></citation>"#,
        "",
        "note",
    );
    let document = Document::new(library(), style, None);
    let rendered = document
        .render(vec![request(&["a"], Some(8)), request(&["a"], Some(9))])
        .unwrap();
    assert_eq!(
        rendered
            .citations
            .iter()
            .map(|item| item.text.as_str())
            .collect::<Vec<_>>(),
        ["8 far", "8 near"]
    );
    assert_eq!(
        document
            .render(vec![request(&["a"], Some(9))])
            .unwrap()
            .citations[0]
            .text,
        "9 far"
    );
    assert_eq!(
        document.render(vec![request(&["a"], Some(0))]).unwrap_err(),
        DocumentError::InvalidNoteNumber
    );
}

#[test]
fn bibliography_layout_and_formatted_content_are_preserved() {
    let style = style(
        r#"<citation><layout><text variable="title"/></layout></citation>"#,
        r#"<bibliography hanging-indent="true" line-spacing="2" entry-spacing="3"><layout><text variable="title" font-style="italic"/></layout></bibliography>"#,
        "in-text",
    );
    let bibliography = Document::new(library(), style, None)
        .full_bibliography()
        .unwrap();
    assert_eq!(
        bibliography.layout,
        Some(BibliographyLayout {
            hanging_indent: true,
            second_field_align: None,
            line_spacing: 2,
            entry_spacing: 3
        })
    );
    let RenderedNode::BibliographyEntry {
        key,
        label,
        content,
    } = &bibliography.tree_nodes()[0]
    else {
        panic!("expected bibliography entry");
    };
    assert_eq!(key, "a");
    assert!(label.is_none());
    fn italic(nodes: &[RenderedNode]) -> Option<&str> {
        nodes.iter().find_map(|node| match node {
            RenderedNode::Text { text, formatting }
                if formatting.font_style == FontStyle::Italic =>
            {
                Some(text.as_str())
            }
            RenderedNode::Element { children, .. } => italic(children),
            _ => None,
        })
    }
    assert_eq!(italic(content), Some("Alpha"));
}

#[test]
fn bibliography_label_and_content_have_distinct_ownership() {
    let style = style(
        r#"<citation><layout><text variable="citation-number"/></layout></citation>"#,
        r#"<bibliography second-field-align="flush"><layout><text variable="citation-number" prefix="[" suffix="]"/><text variable="title"/></layout></bibliography>"#,
        "in-text",
    );
    let bibliography = Document::new(library(), style, None)
        .full_bibliography()
        .unwrap();
    let RenderedNode::BibliographyEntry {
        label: Some(label),
        content,
        ..
    } = &bibliography.tree_nodes()[0]
    else {
        panic!("expected labeled bibliography entry");
    };
    fn text(node: &RenderedNode) -> String {
        match node {
            RenderedNode::Text { text, .. } => text.clone(),
            RenderedNode::Element { children, .. } => children.iter().map(text).collect(),
            _ => String::new(),
        }
    }
    assert_eq!(text(label), "[1]");
    assert_eq!(content.iter().map(text).collect::<String>(), "Alpha");
    assert_eq!(bibliography.text, "[1] Alpha\n[2] Beta");
}

#[test]
fn all_independent_author_year_groups_are_disambiguated() {
    let source = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"].iter().flat_map(|name| {
        [1, 2].map(|index| format!("@book{{{name}{index}, author={{{name}}}, title={{Work {index}}}, year={{2026}}}}"))
    }).collect::<Vec<_>>().join("\n");
    let library = Arc::new(Library::parse_biblatex(&source, RecoveryPolicy::Error).unwrap());
    let style = style(
        r#"<citation disambiguate-add-year-suffix="true"><layout delimiter="; " prefix="(" suffix=")"><group delimiter=", "><names variable="author"/><date variable="issued"><date-part name="year"/></date></group></layout></citation>"#,
        "",
        "note",
    );
    let keys = library
        .keys()
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let requests = vec![request(&keys, None)];
    let rendered = Document::new(library, style, None)
        .render(requests)
        .unwrap();
    assert_eq!(
        rendered.citations[0].text,
        "(Alpha, 2026a; Alpha, 2026b; Beta, 2026a; Beta, 2026b; Gamma, 2026a; Gamma, 2026b; Delta, 2026a; Delta, 2026b; Epsilon, 2026a; Epsilon, 2026b)"
    );
}
