use polars::prelude::*;
use pyo3_polars::derive::polars_expr;
use refkit_core::parse_bibtex_report;

use super::ParseKwargs;
use super::broadcast::parse_value_library_source;
use super::dtypes::{boolean_output, keys_output, parse_report_output, uint32_output};

#[polars_expr(output_type_func=uint32_output)]
fn entry_count(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let output = bibtex
        .iter()
        .map(|value| {
            value.and_then(|source| {
                parse_value_library_source(source, kwargs.recovery.policy())
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
        match parse_value_library_source(source, kwargs.recovery.policy()) {
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
        let report = parse_bibtex_report(source, kwargs.recovery.policy());
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
        let report = parse_bibtex_report(source, kwargs.recovery.policy());
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

#[polars_expr(output_type_func=boolean_output)]
fn can_parse(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let output = bibtex
        .iter()
        .map(|value| {
            value.map(|source| parse_value_library_source(source, kwargs.recovery.policy()).is_ok())
        })
        .collect::<BooleanChunked>();
    Ok(output.into_series())
}

#[polars_expr(output_type_func=boolean_output)]
fn has_diagnostics(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let output = bibtex
        .iter()
        .map(|value| {
            value.map(|source| {
                !parse_bibtex_report(source, kwargs.recovery.policy())
                    .diagnostics
                    .is_empty()
            })
        })
        .collect::<BooleanChunked>();
    Ok(output.into_series())
}
