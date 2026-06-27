use std::sync::{Arc, OnceLock};

use pyo3::prelude::*;
use pyo3::types::PyAny;
use serde_json::{Value, json};

use crate::json_to_py;
use refkit_core::{RenderedFormatting, RenderedNode, RenderedRecord, quoted};

#[pyclass(module = "refkit", skip_from_py_object)]
#[derive(Clone)]
pub struct Rendered {
    record: Arc<RenderedRecord>,
    tree_json: OnceLock<String>,
}

#[pymethods]
impl Rendered {
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

    fn to_text(&self) -> String {
        self.record.text.clone()
    }

    fn to_html(&self) -> String {
        self.record.html.clone()
    }

    fn to_tree(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.tree(py)
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
            serde_json::to_string(&rendered_nodes_to_json(&record.tree_nodes()))
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
            "display": display,
            "meta": meta,
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
        RenderedNode::Transparent { cite_idx, format } => json!({
            "kind": "Transparent",
            "cite_idx": cite_idx,
            "format": format,
        }),
        RenderedNode::BibliographyEntry {
            key,
            first_field,
            children,
        } => json!({
            "kind": "bibliography-entry",
            "key": key,
            "first_field": first_field.as_deref().map(rendered_node_to_json),
            "children": rendered_nodes_to_json(children),
        }),
    }
}

fn formatting_to_json(formatting: &RenderedFormatting) -> Value {
    json!({
        "font_style": formatting.font_style,
        "font_variant": formatting.font_variant,
        "font_weight": formatting.font_weight,
        "text_decoration": formatting.text_decoration,
        "vertical_align": formatting.vertical_align,
    })
}
