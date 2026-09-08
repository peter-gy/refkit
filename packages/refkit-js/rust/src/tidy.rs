use refkit_core::{DuplicateRule, MergeStrategy, TidyError, TidyOptions, TidyWarning};
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

use crate::errors::error;

#[wasm_bindgen]
pub fn validate_tidy_options(options: &str) -> Result<(), JsValue> {
    parse_options(options).map(|_| ())
}

#[wasm_bindgen]
pub fn tidy_bibtex(source: &str, options: &str) -> Result<String, JsValue> {
    let result = refkit_core::tidy_bibtex(source, parse_options(options)?).map_err(tidy_error)?;
    let warnings = result
        .warnings
        .iter()
        .map(|warning| {
            let (rule, message) = match warning {
                TidyWarning::MissingKey { message } => (None, message),
                TidyWarning::DuplicateEntry { rule, message } => (Some(rule.as_str()), message),
            };
            json!({"code": warning.code(), "rule": rule, "message": message})
        })
        .collect::<Vec<_>>();
    let renames = result.renames.iter().map(|rename| json!({
        "entryId": rename.entry_id.index(), "oldKey": rename.old_key, "newKey": rename.new_key
    })).collect::<Vec<_>>();
    Ok(json!({"bibtex": result.bibtex, "warnings": warnings, "renames": renames, "count": result.count}).to_string())
}

fn parse_options(source: &str) -> Result<TidyOptions, JsValue> {
    let value: Value = serde_json::from_str(source).map_err(|value| error("TypeError", value))?;
    let mut options = TidyOptions::default();
    if value.is_null() {
        return Ok(options);
    }
    let values = value
        .as_object()
        .ok_or_else(|| error("TypeError", "tidy options must be an object"))?;
    for (name, value) in values {
        match name.as_str() {
            "omit" => options.omit = strings(value, name)?,
            "curly" => options.curly = boolean(value, name)?,
            "numeric" => options.numeric = boolean(value, name)?,
            "months" => options.months = boolean(value, name)?,
            "space" => options.space = integer(value, name)?,
            "tab" => options.tab = boolean(value, name)?,
            "align" => options.align = default_integer(value, name, TidyOptions::default().align)?,
            "blankLines" => options.blank_lines = boolean(value, name)?,
            "sort" => {
                options.sort = default_strings(
                    value,
                    name,
                    TidyOptions::default().with_sort().sort.unwrap_or_default(),
                )?
            }
            "duplicates" => options.duplicates = duplicates(value)?,
            "merge" => options.merge = merge(value)?,
            "stripEnclosingBraces" => options.strip_enclosing_braces = boolean(value, name)?,
            "dropAllCaps" => options.drop_all_caps = boolean(value, name)?,
            "escape" => options.escape = boolean(value, name)?,
            "sortFields" => {
                options.sort_fields = default_strings(
                    value,
                    name,
                    TidyOptions::default()
                        .with_sort_fields()
                        .sort_fields
                        .unwrap_or_default(),
                )?
            }
            "stripComments" => options.strip_comments = boolean(value, name)?,
            "trailingCommas" => options.trailing_commas = boolean(value, name)?,
            "encodeUrls" => options.encode_urls = boolean(value, name)?,
            "tidyComments" => options.tidy_comments = boolean(value, name)?,
            "removeEmptyFields" => options.remove_empty_fields = boolean(value, name)?,
            "removeDuplicateFields" => options.remove_duplicate_fields = boolean(value, name)?,
            "generateKeys" => {
                options.generate_keys = match value {
                    Value::Null | Value::Bool(false) => None,
                    Value::Bool(true) => TidyOptions::default().with_generate_keys().generate_keys,
                    Value::String(value) => Some(value.clone()),
                    _ => {
                        return Err(error(
                            "TypeError",
                            "generateKeys must be a boolean, string, or null",
                        ));
                    }
                }
            }
            "maxAuthors" => {
                options.max_authors = if value.is_null() {
                    None
                } else {
                    Some(integer(value, name)?)
                }
            }
            "lowercase" => options.lowercase = boolean(value, name)?,
            "enclosingBraces" => {
                options.enclosing_braces = default_strings(value, name, vec!["title".to_string()])?
            }
            "removeBraces" => {
                options.remove_braces = default_strings(value, name, vec!["title".to_string()])?
            }
            "wrap" => {
                options.wrap =
                    default_integer(value, name, TidyOptions::default().with_wrap().wrap)?
            }
            _ => return Err(error("RangeError", format!("unknown tidy option {name:?}"))),
        }
    }
    Ok(options)
}

fn boolean(value: &Value, name: &str) -> Result<bool, JsValue> {
    value
        .as_bool()
        .ok_or_else(|| error("TypeError", format!("{name} must be a boolean")))
}

fn integer(value: &Value, name: &str) -> Result<usize, JsValue> {
    let value = value
        .as_u64()
        .ok_or_else(|| error("TypeError", format!("{name} must be a nonnegative integer")))?;
    usize::try_from(value).map_err(|_| error("RangeError", format!("{name} is too large")))
}

fn default_integer(
    value: &Value,
    name: &str,
    default: Option<usize>,
) -> Result<Option<usize>, JsValue> {
    match value {
        Value::Null | Value::Bool(false) => Ok(None),
        Value::Bool(true) => Ok(default),
        _ => integer(value, name).map(Some),
    }
}

fn strings(value: &Value, name: &str) -> Result<Vec<String>, JsValue> {
    if value.is_null() {
        return Ok(Vec::new());
    }
    let values = value
        .as_array()
        .ok_or_else(|| error("TypeError", format!("{name} must be an array of strings")))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_string)
                .ok_or_else(|| error("TypeError", format!("{name} must be an array of strings")))
        })
        .collect()
}

fn default_strings(
    value: &Value,
    name: &str,
    default: Vec<String>,
) -> Result<Option<Vec<String>>, JsValue> {
    match value {
        Value::Null | Value::Bool(false) => Ok(None),
        Value::Bool(true) => Ok(Some(default)),
        _ => strings(value, name).map(Some),
    }
}

fn duplicates(value: &Value) -> Result<Option<Vec<DuplicateRule>>, JsValue> {
    if value.is_null() {
        return Ok(None);
    }
    strings(value, "duplicates")?
        .iter()
        .map(|value| match value.as_str() {
            "doi" => Ok(DuplicateRule::Doi),
            "key" => Ok(DuplicateRule::Key),
            "abstract" => Ok(DuplicateRule::Abstract),
            "citation" => Ok(DuplicateRule::Citation),
            _ => Err(error(
                "RangeError",
                format!("unknown duplicate rule {value:?}"),
            )),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

fn merge(value: &Value) -> Result<Option<MergeStrategy>, JsValue> {
    if value.is_null() {
        return Ok(None);
    }
    let value = value
        .as_str()
        .ok_or_else(|| error("TypeError", "merge must be a string or null"))?;
    match value {
        "first" => Ok(Some(MergeStrategy::First)),
        "last" => Ok(Some(MergeStrategy::Last)),
        "combine" => Ok(Some(MergeStrategy::Combine)),
        "overwrite" => Ok(Some(MergeStrategy::Overwrite)),
        _ => Err(error(
            "RangeError",
            format!("unknown merge strategy {value:?}"),
        )),
    }
}

fn tidy_error(value: TidyError) -> JsValue {
    match value {
        TidyError::Syntax { line, column, byte, character, message } => JsValue::from_str(&json!({
            "name": "TidySyntaxError", "message": message,
            "syntax": {"line": line, "column": column, "byte": byte, "character": character.map(|value| value.to_string())}
        }).to_string()),
        value => error("TidyError", value),
    }
}
