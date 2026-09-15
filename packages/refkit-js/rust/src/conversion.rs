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

pub fn raw_keys(value: Value, to_host: bool) -> Result<Value, String> {
    const KEYS: &[(&str, &str)] = &[
        ("entry_id", "entryId"),
        ("field_id", "fieldId"),
        ("entry_type", "entryType"),
        ("retained_id", "retainedId"),
        ("removed_ids", "removedIds"),
    ];
    match value {
        Value::Object(fields) => Ok(Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| {
                    if !to_host && KEYS.iter().any(|(core, _)| key == *core) {
                        return Err(format!("Unknown raw property {key:?}"));
                    }
                    let renamed = KEYS
                        .iter()
                        .find_map(|(core, host)| {
                            let (from, to) = if to_host { (core, host) } else { (host, core) };
                            (key == *from).then_some(*to)
                        })
                        .map(str::to_string)
                        .unwrap_or(key);
                    Ok((renamed, raw_keys(value, to_host)?))
                })
                .collect::<Result<_, _>>()?,
        )),
        Value::Array(values) => Ok(Value::Array(
            values
                .into_iter()
                .map(|value| raw_keys(value, to_host))
                .collect::<Result<_, _>>()?,
        )),
        value => Ok(value),
    }
}

pub fn validation(report: &refkit_core::ValidationReport) -> Value {
    fn target(value: &refkit_core::ValidationTarget) -> Value {
        json!({"entry": value.entry, "path": value.path, "entryId": value.entry_id, "fieldId": value.field_id, "span": value.span.as_ref().map(|span| [span.start, span.end])})
    }
    json!({"profile": report.profile, "valid": report.is_valid(), "issues": report.issues.iter().map(|issue| json!({
        "code": issue.code, "severity": issue.severity, "target": target(&issue.target), "related": issue.related.iter().map(target).collect::<Vec<_>>(), "message": issue.message, "suggestion": issue.suggestion
    })).collect::<Vec<_>>()})
}

pub fn record(value: &EntryRecord) -> Value {
    record_keys(json!(value), true).expect("core records use canonical field names")
}

pub fn record_keys(value: Value, to_host: bool) -> Result<Value, wasm_bindgen::JsValue> {
    const KEYS: &[(&str, &str)] = &[
        ("entry_type", "entryType"),
        ("event_date", "eventDate"),
        ("original_date", "originalDate"),
        ("volume_total", "volumeTotal"),
        ("page_range", "pageRange"),
        ("page_total", "pageTotal"),
        ("time_range", "timeRange"),
        ("archive_location", "archiveLocation"),
        ("call_number", "callNumber"),
        ("abstract_text", "abstractText"),
        ("comma_suffix", "commaSuffix"),
        ("given_initials", "givenInitials"),
        ("prefix_initials", "prefixInitials"),
        ("use_prefix", "usePrefix"),
        ("non_dropping_particle", "nonDroppingParticle"),
    ];
    match value {
        Value::Object(fields) => Ok(Value::Object(
            fields
                .into_iter()
                .map(|(key, value)| {
                    if key == "extensions" || key == "identifiers" {
                        return Ok((key, value));
                    }
                    if !to_host && KEYS.iter().any(|(core, _)| key == *core) {
                        return Err(crate::errors::error(
                            "RangeError",
                            format!("unknown record property {key:?}"),
                        ));
                    }
                    let renamed = KEYS
                        .iter()
                        .find_map(|(core, host)| {
                            let (from, to) = if to_host { (core, host) } else { (host, core) };
                            (key == *from).then_some(*to)
                        })
                        .map(str::to_string)
                        .unwrap_or(key);
                    Ok((renamed, record_keys(value, to_host)?))
                })
                .collect::<Result<_, _>>()?,
        )),
        Value::Array(values) => Ok(Value::Array(
            values
                .into_iter()
                .map(|value| record_keys(value, to_host))
                .collect::<Result<_, _>>()?,
        )),
        value => Ok(value),
    }
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
