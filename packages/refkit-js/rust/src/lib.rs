mod conversion;
mod document;
mod errors;
mod library;
mod raw;
mod tidy;

pub use document::{NativeDocument, NativeStyle};
pub use library::NativeLibrary;
pub use raw::NativeRawDocument;
pub use tidy::{tidy_bibtex, validate_tidy_options};

use serde_json::json;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn is_bundled_locale(code: &str) -> bool {
    refkit_core::is_bundled_locale(code)
}

#[wasm_bindgen]
pub fn decode_bibliography(bytes: &[u8]) -> String {
    let decoded = refkit_core::decode_bibliography(bytes);
    let diagnostic = (decoded.encoding == refkit_core::TextEncoding::Windows1252).then(|| json!({
        "code": "text_encoding", "severity": "warning", "action": "decoded",
        "span": null, "entry": null, "field": null,
        "message": "Decoded as Windows-1252-compatible text because the source is not valid UTF-8. Spans refer to the decoded UTF-8 source and writes use UTF-8"
    }));
    json!({"text": decoded.text, "diagnostic": diagnostic}).to_string()
}

#[wasm_bindgen]
pub fn build_info() -> String {
    json!({
        "version": env!("CARGO_PKG_VERSION"),
        "buildMode": if cfg!(debug_assertions) { "debug" } else { "release" },
        "target": "wasm32-unknown-unknown"
    })
    .to_string()
}
