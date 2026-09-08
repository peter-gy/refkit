use refkit_core::{DocumentError, LibraryError, StyleError};
use serde_json::json;
use wasm_bindgen::JsValue;

pub fn error(name: &str, message: impl ToString) -> JsValue {
    JsValue::from_str(&json!({"name": name, "message": message.to_string()}).to_string())
}

pub fn library_error(value: LibraryError) -> JsValue {
    match &value {
        LibraryError::Biblatex(failure) | LibraryError::HayagrivaYaml(failure) => {
            JsValue::from_str(
                &json!({
                    "name": "ParseError", "message": value.to_string(),
                    "diagnostics": crate::conversion::diagnostics(&failure.diagnostics)
                })
                .to_string(),
            )
        }
        LibraryError::Selector(_) => error("RangeError", value),
    }
}

pub fn document_error(value: DocumentError) -> JsValue {
    let name = match &value {
        DocumentError::MissingReference(_) => "MissingReferenceError",
        DocumentError::Render(_) => "RefkitError",
        _ => "RangeError",
    };
    error(name, value)
}

pub fn style_error(value: StyleError) -> JsValue {
    let name = if matches!(value, StyleError::CachePoisoned) {
        "RefkitError"
    } else {
        "RangeError"
    };
    error(name, value)
}
