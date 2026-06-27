use polars::prelude::*;
use pyo3_polars::derive::polars_expr;
use refkit_core::{NormalizedEntry, NormalizedValue, parse_bibtex_report_source};
use serde_json::Value;

use super::ParseKwargs;
use super::broadcast::parse_value_library_source;
use super::dtypes::{keys_output, parse_report_output};

#[polars_expr(output_type=UInt32)]
fn entry_count(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let output = bibtex
        .iter()
        .map(|value| {
            value.and_then(|source| {
                parse_value_library_source(source, kwargs.strict)
                    .ok()
                    .and_then(|library| u32::try_from(library.len()).ok())
            })
        })
        .collect::<UInt32Chunked>();
    Ok(output.into_series())
}

#[polars_expr(output_type_func=keys_output)]
fn keys(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let mut builder = ListStringChunkedBuilder::new("keys".into(), bibtex.len(), bibtex.len() * 2);

    for value in bibtex.iter() {
        let Some(source) = value else {
            builder.append_null();
            continue;
        };
        match parse_value_library_source(source, kwargs.strict) {
            Ok(library) => {
                builder.append_values_iter(library.keys().iter().map(String::as_str));
            }
            Err(_) => builder.append_null(),
        }
    }

    Ok(builder.finish().into_series())
}

#[polars_expr(output_type_func=keys_output)]
fn diagnostics(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let mut builder =
        ListStringChunkedBuilder::new("diagnostics".into(), bibtex.len(), bibtex.len());

    for value in bibtex.iter() {
        let Some(source) = value else {
            builder.append_null();
            continue;
        };
        let report = parse_bibtex_report_source(source, kwargs.strict);
        builder.append_values_iter(report.diagnostics.iter().map(String::as_str));
    }

    Ok(builder.finish().into_series())
}

#[polars_expr(output_type_func=parse_report_output)]
fn parse_report(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let mut ok = Vec::with_capacity(bibtex.len());
    let mut entry_count = Vec::with_capacity(bibtex.len());
    let mut keys = ListStringChunkedBuilder::new("keys".into(), bibtex.len(), bibtex.len() * 2);
    let mut diagnostics =
        ListStringChunkedBuilder::new("diagnostics".into(), bibtex.len(), bibtex.len());

    for value in bibtex.iter() {
        let Some(source) = value else {
            ok.push(None);
            entry_count.push(None);
            keys.append_null();
            diagnostics.append_null();
            continue;
        };
        let report = parse_bibtex_report_source(source, kwargs.strict);
        ok.push(Some(report.ok));
        entry_count.push(
            report
                .entry_count
                .and_then(|count| u32::try_from(count).ok()),
        );
        match report.keys {
            Some(keys_value) => keys.append_values_iter(keys_value.iter().map(String::as_str)),
            None => keys.append_null(),
        }
        diagnostics.append_values_iter(report.diagnostics.iter().map(String::as_str));
    }

    let fields = [
        BooleanChunked::from_iter_options("ok".into(), ok.into_iter()).into_series(),
        UInt32Chunked::from_iter_options("entry_count".into(), entry_count.into_iter())
            .into_series(),
        keys.finish().into_series(),
        diagnostics.finish().into_series(),
    ];
    Ok(
        StructChunked::from_series("parse_report".into(), bibtex.len(), fields.iter())?
            .into_series(),
    )
}

#[polars_expr(output_type=Boolean)]
fn can_parse(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let output = bibtex
        .iter()
        .map(|value| value.map(|source| parse_value_library_source(source, kwargs.strict).is_ok()))
        .collect::<BooleanChunked>();
    Ok(output.into_series())
}

#[polars_expr(output_type=Boolean)]
fn has_diagnostics(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let output = bibtex
        .iter()
        .map(|value| {
            value.map(|source| {
                !parse_bibtex_report_source(source, kwargs.strict)
                    .diagnostics
                    .is_empty()
            })
        })
        .collect::<BooleanChunked>();
    Ok(output.into_series())
}

#[polars_expr(output_type=String)]
fn to_hayagriva_json(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let output = bibtex
        .iter()
        .map(|value| {
            let source = value?;
            let library = parse_value_library_source(source, kwargs.strict).ok()?;
            library.normalized_entries().ok().and_then(|entries| {
                serde_json::to_string(&normalized_entries_to_json(&entries)).ok()
            })
        })
        .collect::<StringChunked>();
    Ok(output.into_series())
}

fn normalized_entries_to_json(entries: &[NormalizedEntry]) -> Vec<Value> {
    entries
        .iter()
        .map(|entry| normalized_value_to_json(&entry.value))
        .collect()
}

fn normalized_value_to_json(value: &NormalizedValue) -> Value {
    match value {
        NormalizedValue::Null => Value::Null,
        NormalizedValue::Bool(value) => Value::Bool(*value),
        NormalizedValue::Number(value) => Value::Number(value.clone()),
        NormalizedValue::String(value) => Value::String(value.clone()),
        NormalizedValue::Array(values) => {
            Value::Array(values.iter().map(normalized_value_to_json).collect())
        }
        NormalizedValue::Object(values) => Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), normalized_value_to_json(value)))
                .collect(),
        ),
    }
}
