use std::sync::Arc;

use refkit_core::{
    BibliographyFormat, CodecError, ConversionIssue, Library, LossPolicy, RecoveryPolicy,
};
use wasm_bindgen::prelude::*;

use crate::NativeLibrary;
use crate::errors::{codec_error, error};

fn format(value: &str) -> Result<BibliographyFormat, JsValue> {
    value
        .parse()
        .map_err(|value: CodecError| error("RangeError", value))
}
fn loss(value: &str) -> Result<LossPolicy, JsValue> {
    value
        .parse()
        .map_err(|value: CodecError| error("RangeError", value))
}
fn recovery(value: &str) -> Result<RecoveryPolicy, JsValue> {
    match value {
        "error" => Ok(RecoveryPolicy::Error),
        "report" => Ok(RecoveryPolicy::Report),
        _ => Err(error("RangeError", "recovery must be 'error' or 'report'")),
    }
}

#[wasm_bindgen]
pub struct NativeDecodeReport {
    library: Arc<Library>,
    format: BibliographyFormat,
    issues: Vec<ConversionIssue>,
}

#[wasm_bindgen]
impl NativeDecodeReport {
    #[wasm_bindgen(getter)]
    pub fn library(&self) -> NativeLibrary {
        NativeLibrary {
            inner: Arc::clone(&self.library),
        }
    }
    #[wasm_bindgen(getter)]
    pub fn format(&self) -> String {
        self.format.as_str().into()
    }
    pub fn issues(&self) -> String {
        serde_json::to_string(&self.issues).expect("owned issues serialize")
    }
}

#[wasm_bindgen]
pub fn decode_format(
    source: &str,
    source_format: &str,
    loss_policy: &str,
    recovery_policy: &str,
) -> Result<NativeDecodeReport, JsValue> {
    let result = refkit_core::decode(
        source,
        format(source_format)?,
        loss(loss_policy)?,
        recovery(recovery_policy)?,
    )
    .map_err(codec_error)?;
    Ok(NativeDecodeReport {
        library: Arc::new(result.library),
        format: result.format,
        issues: result.issues,
    })
}

#[wasm_bindgen]
pub fn encode_format(
    library: &NativeLibrary,
    target_format: &str,
    loss_policy: &str,
) -> Result<String, JsValue> {
    let result = refkit_core::encode(&library.inner, format(target_format)?, loss(loss_policy)?)
        .map_err(codec_error)?;
    serde_json::to_string(&result).map_err(|value| error("RefkitError", value))
}

#[wasm_bindgen]
pub fn convert_format(
    source: &str,
    source_format: &str,
    target_format: &str,
    loss_policy: &str,
    recovery_policy: &str,
) -> Result<String, JsValue> {
    let result = refkit_core::convert(
        source,
        format(source_format)?,
        format(target_format)?,
        loss(loss_policy)?,
        recovery(recovery_policy)?,
    )
    .map_err(codec_error)?;
    Ok(serde_json::json!({"sourceFormat": result.source_format, "targetFormat": result.target_format, "text": result.text, "issues": result.issues, "diagnostics": crate::conversion::diagnostics(&result.diagnostics)}).to_string())
}
