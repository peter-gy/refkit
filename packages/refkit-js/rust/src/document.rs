use std::collections::HashSet;
use std::sync::Arc;

use refkit_core::{
    CitationRequest, Cite, Document, PreparedStyle, load_prepared_style, prepare_style_from_xml,
};
use serde::Deserialize;
use serde_json::{Map, json};
use wasm_bindgen::prelude::*;

use crate::NativeLibrary;
use crate::conversion::rendered;
use crate::errors::{document_error, error, style_error};

#[wasm_bindgen]
pub struct NativeStyle {
    id: String,
    inner: Arc<PreparedStyle>,
}

#[wasm_bindgen]
impl NativeStyle {
    pub fn load(name: &str) -> Result<NativeStyle, JsValue> {
        Ok(Self {
            id: name.to_string(),
            inner: load_prepared_style(name).map_err(style_error)?,
        })
    }

    pub fn from_xml(xml: &str) -> Result<NativeStyle, JsValue> {
        Ok(Self {
            id: "xml".to_string(),
            inner: Arc::new(prepare_style_from_xml(xml).map_err(style_error)?),
        })
    }

    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn title(&self) -> String {
        self.inner.title().to_string()
    }
}

#[wasm_bindgen]
pub struct NativeDocument {
    inner: Document,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireCitation {
    id: String,
    items: Vec<WireCite>,
    note_number: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireCite {
    key: String,
    locator: Option<String>,
    label: Option<String>,
}

fn requests(source: &str) -> Result<(Vec<String>, Vec<CitationRequest>), JsValue> {
    let citations: Vec<WireCitation> =
        serde_json::from_str(source).map_err(|value| error("TypeError", value))?;
    let mut seen = HashSet::new();
    let mut ids = Vec::with_capacity(citations.len());
    let mut requests = Vec::with_capacity(citations.len());
    for citation in citations {
        if !seen.insert(citation.id.clone()) {
            return Err(error(
                "RangeError",
                format!("duplicate citation id {:?}", citation.id),
            ));
        }
        ids.push(citation.id);
        requests.push(CitationRequest::new(
            citation
                .items
                .into_iter()
                .map(|item| Cite::new(item.key, item.locator, item.label))
                .collect(),
            citation.note_number,
        ));
    }
    Ok((ids, requests))
}

#[wasm_bindgen]
impl NativeDocument {
    #[wasm_bindgen(constructor)]
    pub fn new(
        library: &NativeLibrary,
        style: &NativeStyle,
        locale: Option<String>,
    ) -> NativeDocument {
        Self {
            inner: Document::new(Arc::clone(&library.inner), Arc::clone(&style.inner), locale),
        }
    }

    pub fn render(&self, citations: &str) -> Result<String, JsValue> {
        let (ids, requests) = requests(citations)?;
        let output = self.inner.render(requests).map_err(document_error)?;
        let citations: Map<_, _> = ids
            .iter()
            .cloned()
            .zip(output.citations.iter().map(rendered))
            .collect();
        Ok(json!({"citationOrder": ids, "citations": citations, "bibliography": rendered(&output.bibliography)}).to_string())
    }

    pub fn cited_bibliography(&self, citations: &str) -> Result<String, JsValue> {
        let (_, requests) = requests(citations)?;
        self.inner
            .cited_bibliography(requests)
            .map(|value| rendered(&value).to_string())
            .map_err(document_error)
    }

    pub fn full_bibliography(&self) -> Result<String, JsValue> {
        self.inner
            .full_bibliography()
            .map(|value| rendered(&value).to_string())
            .map_err(document_error)
    }
}
