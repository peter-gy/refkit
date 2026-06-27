use hayagriva::{Entry as HayEntry, Library as HayLibrary};
use serde_json::{Value, json};

use super::{NormalizedEntry, NormalizedValue};

pub(super) fn normalized_entries(library: &HayLibrary) -> Result<Vec<NormalizedEntry>, String> {
    library.iter().map(entry_to_normalized_entry).collect()
}

fn entry_to_normalized_entry(entry: &HayEntry) -> Result<NormalizedEntry, String> {
    let mut value = serde_json::to_value(entry).map_err(|err| err.to_string())?;
    if let Some(map) = value.as_object_mut() {
        map.insert("id".to_string(), json!(entry.key()));
        map.insert("key".to_string(), json!(entry.key()));
    }
    Ok(NormalizedEntry {
        value: normalized_value_from_json(value),
    })
}

fn normalized_value_from_json(value: Value) -> NormalizedValue {
    match value {
        Value::Null => NormalizedValue::Null,
        Value::Bool(value) => NormalizedValue::Bool(value),
        Value::Number(value) => NormalizedValue::Number(value),
        Value::String(value) => NormalizedValue::String(value),
        Value::Array(values) => {
            NormalizedValue::Array(values.into_iter().map(normalized_value_from_json).collect())
        }
        Value::Object(values) => NormalizedValue::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, normalized_value_from_json(value)))
                .collect(),
        ),
    }
}
