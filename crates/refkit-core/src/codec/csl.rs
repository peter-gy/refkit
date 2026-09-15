mod decode;
mod encode;
mod types;

use serde_json::Value;

use super::CodecError;

pub(super) use decode::decode;
pub(super) use encode::encode;
pub(super) use types::{canonical_type, record_type};

fn scalar(value: &Value, field: &str) -> Result<String, CodecError> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        _ => Err(CodecError::new(format!(
            "CSL {field} must be a string or number"
        ))),
    }
}
fn identifier(value: &Value) -> Result<String, CodecError> {
    if let Value::Number(number) = value {
        if let Some(value) = number.as_i64() {
            return Ok(value.to_string());
        }
        if let Some(value) = number.as_u64() {
            return Ok(value.to_string());
        }
        if let Some(value) = number.as_f64() {
            return Ok(value.to_string());
        }
    }
    scalar(value, "id")
}
