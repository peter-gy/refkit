use refkit_core::{DocumentError, LibraryError, ParseFailure, StyleError};
use serde_json::json;
use wasm_bindgen::JsValue;

#[expect(
    clippy::needless_pass_by_value,
    reason = "Result::map_err transfers error ownership into this JavaScript exception conversion boundary."
)]
pub fn codec_error(value: refkit_core::CodecError) -> JsValue {
    JsValue::from_str(&json!({"name": "ConversionError", "message": value.message, "issues": value.issues, "diagnostics": crate::conversion::diagnostics(&value.diagnostics)}).to_string())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Result::map_err transfers error ownership into this JavaScript exception conversion boundary."
)]
pub fn patch_error(value: refkit_core::BibPatchError) -> JsValue {
    JsValue::from_str(&json!({"name": "PatchError", "message": value.message, "code": value.code, "operation": value.operation}).to_string())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Result::map_err transfers error ownership into this JavaScript exception conversion boundary."
)]
pub fn merge_error(value: refkit_core::MergeError) -> JsValue {
    JsValue::from_str(
        &json!({"name": "MergeError", "message": value.message, "code": value.code}).to_string(),
    )
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "This formatting boundary accepts both owned errors and borrowed messages from JavaScript conversion callers."
)]
pub fn error(name: &str, message: impl ToString) -> JsValue {
    JsValue::from_str(&json!({"name": name, "message": message.to_string()}).to_string())
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "Result::map_err transfers error ownership into this JavaScript exception conversion boundary."
)]
pub fn parse_error(value: ParseFailure) -> JsValue {
    JsValue::from_str(
        &json!({
            "name": "ParseError", "message": value.to_string(),
            "diagnostics": crate::conversion::diagnostics(&value.diagnostics)
        })
        .to_string(),
    )
}

pub fn library_error(value: LibraryError) -> JsValue {
    match value {
        LibraryError::Biblatex(failure) | LibraryError::HayagrivaYaml(failure) => {
            parse_error(failure)
        }
        LibraryError::Selector(_) | LibraryError::Record(_) => error("RangeError", value),
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
