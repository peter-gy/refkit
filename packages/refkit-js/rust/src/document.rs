use std::collections::HashSet;
use std::sync::Arc;

use refkit_core::{
    CitationRequest, Cite, Document, PreparedStyle, load_prepared_style, prepare_style_from_xml,
    style_catalog,
};
use serde::Deserialize;
use serde_json::{Map, json};
use wasm_bindgen::prelude::*;

use crate::NativeLibrary;
use crate::conversion::rendered;
use crate::errors::{document_error, error, style_error};

#[wasm_bindgen]
/// A prepared style shared by WebAssembly rendering documents.
pub struct NativeStyle {
    id: String,
    inner: Arc<PreparedStyle>,
}

#[wasm_bindgen]
impl NativeStyle {
    #[must_use]
    /// Return the bundled style catalog and aliases as JSON.
    pub fn list() -> String {
        let styles: Vec<_> = style_catalog()
            .into_iter()
            .map(|style| {
                json!({
                    "name": style.name, "aliases": style.aliases,
                    "title": style.title, "cslId": style.csl_id,
                })
            })
            .collect();
        json!(styles).to_string()
    }

    /// Load a bundled style by name or alias.
    ///
    /// # Errors
    /// Rejects unknown style names and unavailable prepared-style cache state.
    pub fn load(name: &str) -> Result<NativeStyle, JsValue> {
        Ok(Self {
            id: name.to_string(),
            inner: load_prepared_style(name).map_err(style_error)?,
        })
    }

    /// Prepare supplied CSL XML and an optional parent style.
    ///
    /// # Errors
    /// Rejects invalid XML, unresolved or mismatched parents, invalid macros, and resource limits.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "wasm-bindgen transfers optional JavaScript strings as owned Option<String> values at this exported boundary."
    )]
    pub fn from_xml(xml: &str, parent_xml: Option<String>) -> Result<NativeStyle, JsValue> {
        Ok(Self {
            id: "xml".to_string(),
            inner: Arc::new(
                prepare_style_from_xml(xml, parent_xml.as_deref()).map_err(style_error)?,
            ),
        })
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    /// Return the supplied bundled name, or `xml` for a custom style.
    pub fn id(&self) -> String {
        self.id.clone()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    /// Return the prepared style title.
    pub fn title(&self) -> String {
        self.inner.title().to_string()
    }

    #[wasm_bindgen(getter)]
    #[must_use]
    /// Return the style's CSL identifier.
    pub fn csl_id(&self) -> String {
        self.inner.csl_id().to_string()
    }
}

#[wasm_bindgen]
/// Prepared rendering inputs with fresh citation processor state for each operation.
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
    purpose: String,
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
                .map(|item| {
                    Ok(Cite::new(
                        item.key,
                        item.locator,
                        item.label,
                        item.purpose.parse().map_err(document_error)?,
                    ))
                })
                .collect::<Result<Vec<_>, JsValue>>()?,
            citation.note_number,
        ));
    }
    Ok((ids, requests))
}

#[wasm_bindgen]
impl NativeDocument {
    #[wasm_bindgen(constructor)]
    #[must_use]
    /// Prepare a document from shared library and style handles and an optional locale.
    pub fn new(
        library: &NativeLibrary,
        style: &NativeStyle,
        locale: Option<String>,
    ) -> NativeDocument {
        Self {
            inner: Document::new(Arc::clone(&library.inner), Arc::clone(&style.inner), locale),
        }
    }

    /// Render an ordered JSON citation sequence and its bibliography to a JSON result.
    ///
    /// # Errors
    /// Rejects malformed requests, duplicate citation IDs, invalid items, missing keys, and rendering failures.
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

    /// Render the bibliography for an ordered JSON citation sequence.
    ///
    /// # Errors
    /// Rejects malformed requests, duplicate citation IDs, invalid items, missing keys, and rendering failures.
    pub fn cited_bibliography(&self, citations: &str) -> Result<String, JsValue> {
        let (_, requests) = requests(citations)?;
        self.inner
            .cited_bibliography(requests)
            .map(|value| rendered(&value).to_string())
            .map_err(document_error)
    }

    /// Render a bibliography containing every library record.
    ///
    /// # Errors
    /// Returns the core rendering failure when the prepared inputs cannot be rendered.
    pub fn full_bibliography(&self) -> Result<String, JsValue> {
        self.inner
            .full_bibliography()
            .map(|value| rendered(&value).to_string())
            .map_err(document_error)
    }
}
