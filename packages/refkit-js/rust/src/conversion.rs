use refkit_core::{
    Diagnostic, EntryRecord, RenderedFormatting, RenderedMeta, RenderedNode, RenderedRecord,
};
use serde_json::{Value, json};

pub fn diagnostics(values: &[Diagnostic]) -> Value {
    json!(values.iter().map(|value| json!({
        "code": value.code, "severity": value.severity.as_str(), "action": value.action.as_str(),
        "span": value.span.as_ref().map(|span| [span.start, span.end]),
        "entry": value.entry, "field": value.field, "message": value.message
    })).collect::<Vec<_>>())
}

pub fn record(value: &EntryRecord) -> Value {
    json!({"key": value.key, "entryType": value.entry_type, "title": value.title,
        "date": value.date, "volume": value.volume, "doi": value.doi,
        "parents": value.parents.iter().map(record).collect::<Vec<_>>()})
}

pub fn rendered(value: &RenderedRecord) -> Value {
    let layout = value.layout.map(|layout| {
        json!({
            "hangingIndent": layout.hanging_indent,
            "secondFieldAlign": layout.second_field_align.map(|align| align.as_str()),
            "lineSpacing": layout.line_spacing, "entrySpacing": layout.entry_spacing
        })
    });
    json!({"text": value.text, "html": value.html, "layout": layout,
        "tree": value.tree_nodes().iter().map(node).collect::<Vec<_>>()})
}

fn formatting(value: &RenderedFormatting) -> Value {
    json!({"fontStyle": value.font_style.as_str(), "fontVariant": value.font_variant.as_str(),
        "fontWeight": value.font_weight.as_str(), "textDecoration": value.text_decoration.as_str(),
        "verticalAlign": value.vertical_align.as_str()})
}

fn metadata(value: &RenderedMeta) -> Value {
    match value {
        RenderedMeta::Names { roles } => json!({"kind": "Names", "roles": roles}),
        RenderedMeta::Name { role, index } => json!({"kind": "Name", "role": role, "index": index}),
        RenderedMeta::Entry { key, item_index } => {
            json!({"kind": "Entry", "key": key, "itemIndex": item_index})
        }
        RenderedMeta::Date => json!({"kind": "Date"}),
        RenderedMeta::Text => json!({"kind": "Text"}),
        RenderedMeta::Number => json!({"kind": "Number"}),
        RenderedMeta::Label => json!({"kind": "Label"}),
        RenderedMeta::CitationNumber => json!({"kind": "CitationNumber"}),
        RenderedMeta::CitationLabel => json!({"kind": "CitationLabel"}),
    }
}

fn node(value: &RenderedNode) -> Value {
    match value {
        RenderedNode::Text {
            text,
            formatting: format,
        } => json!({"kind": "Text", "text": text, "formatting": formatting(format)}),
        RenderedNode::Element {
            display,
            meta,
            children,
        } => json!({"kind": "Element",
            "display": display.map(|display| display.as_str()), "meta": meta.as_ref().map(metadata),
            "children": children.iter().map(node).collect::<Vec<_>>()}),
        RenderedNode::Markup { value } => json!({"kind": "Markup", "value": value}),
        RenderedNode::Link {
            text,
            url,
            formatting: format,
        } => json!({"kind": "Link", "text": text, "url": url, "formatting": formatting(format)}),
        RenderedNode::Transparent {
            cite_idx,
            formatting: format,
        } => json!({"kind": "Transparent", "citeIdx": cite_idx, "formatting": formatting(format)}),
        RenderedNode::BibliographyEntry {
            key,
            label,
            content,
        } => json!({"kind": "bibliography-entry", "key": key,
            "label": label.as_deref().map(node), "content": content.iter().map(node).collect::<Vec<_>>()}),
    }
}
