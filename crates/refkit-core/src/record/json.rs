use std::fmt;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};

use super::{EntryRecord, RecordError, validate_record_source};

struct StrictValue(Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct JsonVisitor;
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = StrictValue;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("JSON data with unique object keys")
            }

            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Null))
            }
            fn visit_bool<E: de::Error>(self, value: bool) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Bool(value)))
            }
            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(value.into())))
            }
            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::Number(value.into())))
            }
            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Self::Value, E> {
                Number::from_f64(value)
                    .map(|value| StrictValue(Value::Number(value)))
                    .ok_or_else(|| E::custom("numbers must be finite"))
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(value.to_string())))
            }
            fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(value)))
            }

            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(StrictValue(value)) = sequence.next_element()? {
                    values.push(value);
                }
                Ok(StrictValue(Value::Array(values)))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom(format!("duplicate JSON field {key:?}")));
                    }
                    let StrictValue(value) = map.next_value()?;
                    values.insert(key, value);
                }
                Ok(StrictValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(JsonVisitor)
    }
}

pub(crate) fn parse_json(source: &str) -> Result<Value, RecordError> {
    validate_record_source(source)?;
    let mut decoder = serde_json::Deserializer::from_str(source);
    decoder.disable_recursion_limit();
    let StrictValue(value) = StrictValue::deserialize(&mut decoder)
        .map_err(|error| RecordError::new("records", error))?;
    decoder
        .end()
        .map_err(|error| RecordError::new("records", error))?;
    Ok(value)
}

pub(crate) fn decode_records(source: &str, archive: bool) -> Result<Vec<EntryRecord>, RecordError> {
    let mut value = parse_json(source)?;
    if archive {
        let fields = value
            .as_object_mut()
            .ok_or_else(|| RecordError::new("records", "record snapshot must be an object"))?;
        if fields
            .keys()
            .any(|key| key != "schema_version" && key != "records")
        {
            return Err(RecordError::new("records", "unknown record snapshot field"));
        }
        if fields.get("schema_version").and_then(Value::as_u64) != Some(1) {
            return Err(RecordError::new(
                "schema_version",
                "expected record schema version 1",
            ));
        }
        value = fields
            .remove("records")
            .ok_or_else(|| RecordError::new("records", "record snapshot requires records"))?;
    }
    let records = value
        .as_array()
        .ok_or_else(|| RecordError::new("records", "expected an array of records"))?;
    if records.len() > 100_000 {
        return Err(RecordError::new("records", "record count exceeds 100000"));
    }
    let mut pending: Vec<_> = records.iter().map(|record| (record, 0usize)).collect();
    let mut visited = 0usize;
    while let Some((record, depth)) = pending.pop() {
        visited += 1;
        if depth > 64 || visited > 100_000 {
            return Err(RecordError::new(
                "records",
                "record graph exceeds resource limits",
            ));
        }
        if let Some(parents) = record.get("parents").and_then(Value::as_array) {
            if visited + pending.len() + parents.len() > 100_000 {
                return Err(RecordError::new(
                    "records",
                    "record graph exceeds resource limits",
                ));
            }
            pending.extend(parents.iter().map(|parent| (parent, depth + 1)));
        }
    }
    serde_json::from_value(value).map_err(|error| RecordError::new("records", error))
}
