use std::sync::{Arc, OnceLock};

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict};
use serde_json::{Value, json};

use crate::conversion::json_to_py;
use refkit_core::{RenderedFormatting, RenderedMeta, RenderedNode, RenderedRecord};

use crate::repr::quoted;

#[pyclass(module = "refkit", skip_from_py_object)]
#[derive(Clone)]
pub struct Rendered {
    record: Arc<RenderedRecord>,
    tree_json: OnceLock<String>,
}

#[pymethods]
impl Rendered {
    #[getter]
    fn layout(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        let Some(layout) = &self.record.layout else {
            return Ok(None);
        };
        let value = PyDict::new(py);
        value.set_item("hanging_indent", layout.hanging_indent)?;
        value.set_item(
            "second_field_align",
            layout
                .second_field_align
                .as_ref()
                .map(|value| value.as_str()),
        )?;
        value.set_item("line_spacing", layout.line_spacing)?;
        value.set_item("entry_spacing", layout.entry_spacing)?;
        Ok(Some(value.into_any().unbind()))
    }

    #[getter]
    fn text(&self) -> String {
        self.record.text.clone()
    }

    #[getter]
    fn html(&self) -> String {
        self.record.html.clone()
    }

    #[getter]
    fn tree(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let payload = self.tree_json(py);
        json_to_py(py, &payload)
    }

    fn __repr__(&self) -> String {
        format!("Rendered(text={})", quoted(&preview(&self.record.text)))
    }
}

impl Rendered {
    pub(crate) fn new(record: RenderedRecord) -> Self {
        Self {
            record: Arc::new(record),
            tree_json: OnceLock::new(),
        }
    }

    pub(crate) fn from_record(record: RenderedRecord) -> Self {
        Self::new(record)
    }

    fn tree_json(&self, py: Python<'_>) -> String {
        if let Some(payload) = self.tree_json.get() {
            return payload.clone();
        }

        let record = Arc::clone(&self.record);
        let payload = py.detach(move || {
            serde_json::to_string(&rendered_nodes_to_json(record.tree_nodes()))
                .expect("rendered tree should serialize to Python JSON payload")
        });
        self.tree_json.get_or_init(|| payload).clone()
    }
}

fn preview(value: &str) -> String {
    const LIMIT: usize = 60;
    if value.chars().count() <= LIMIT {
        return value.to_string();
    }
    let mut output: String = value.chars().take(LIMIT).collect();
    output.push_str("...");
    output
}

fn rendered_nodes_to_json(nodes: &[RenderedNode]) -> Vec<Value> {
    nodes.iter().map(rendered_node_to_json).collect()
}

fn rendered_node_to_json(node: &RenderedNode) -> Value {
    match node {
        RenderedNode::Text { text, formatting } => json!({
            "kind": "Text",
            "text": text,
            "formatting": formatting_to_json(formatting),
        }),
        RenderedNode::Element {
            display,
            meta,
            children,
        } => json!({
            "kind": "Element",
            "display": display.as_ref().map(|display| display.as_str()),
            "meta": meta.as_ref().map(meta_to_json),
            "children": rendered_nodes_to_json(children),
        }),
        RenderedNode::Markup { value } => json!({
            "kind": "Markup",
            "value": value,
        }),
        RenderedNode::Link {
            text,
            url,
            formatting,
        } => json!({
            "kind": "Link",
            "text": text,
            "url": url,
            "formatting": formatting_to_json(formatting),
        }),
        RenderedNode::Transparent {
            cite_idx,
            formatting,
        } => json!({
            "kind": "Transparent",
            "cite_idx": cite_idx,
            "formatting": formatting_to_json(formatting),
        }),
        RenderedNode::BibliographyEntry {
            key,
            label,
            content,
        } => json!({
            "kind": "bibliography-entry",
            "key": key,
            "label": label.as_deref().map(rendered_node_to_json),
            "content": rendered_nodes_to_json(content),
        }),
    }
}

fn formatting_to_json(formatting: &RenderedFormatting) -> Value {
    json!({
        "font_style": formatting.font_style.as_str(),
        "font_variant": formatting.font_variant.as_str(),
        "font_weight": formatting.font_weight.as_str(),
        "text_decoration": formatting.text_decoration.as_str(),
        "vertical_align": formatting.vertical_align.as_str(),
    })
}

fn meta_to_json(meta: &RenderedMeta) -> Value {
    match meta {
        RenderedMeta::Entry { key, item_index } => {
            json!({"kind": "Entry", "key": key, "item_index": item_index})
        }
        RenderedMeta::Names { roles } => json!({"kind": "Names", "roles": roles}),
        RenderedMeta::Name { role, index } => json!({"kind": "Name", "role": role, "index": index}),
        RenderedMeta::Date => json!({"kind": "Date"}),
        RenderedMeta::Text => json!({"kind": "Text"}),
        RenderedMeta::Number => json!({"kind": "Number"}),
        RenderedMeta::Label => json!({"kind": "Label"}),
        RenderedMeta::CitationNumber => json!({"kind": "CitationNumber"}),
        RenderedMeta::CitationLabel => json!({"kind": "CitationLabel"}),
    }
}
