use polars::prelude::*;
use polars_core::chunked_array::builder::{AnonymousOwnedListBuilder, ListBuilderTrait};
use pyo3_polars::derive::polars_expr;
use refkit_core::{Diagnostic, parse_bibtex_report};

use super::ParseKwargs;
use super::broadcast::parse_value_library_source;
use super::dtypes::{
    boolean_output, diagnostic_struct_dtype, diagnostics_output, keys_output, parse_report_output,
    uint32_output, with_struct_validity,
};

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

#[polars_expr(output_type_func=diagnostics_output)]
fn diagnostics(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let reports = bibtex
        .iter()
        .map(|source| source.map(|source| parse_bibtex_report(source, kwargs.recovery.policy())))
        .collect::<Vec<_>>();
    diagnostic_lists_to_series(
        "diagnostics",
        reports
            .iter()
            .map(|report| report.as_ref().map(|report| report.diagnostics.as_slice())),
    )
}

#[polars_expr(output_type_func=parse_report_output)]
fn parse_report(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let reports = bibtex
        .iter()
        .map(|source| source.map(|source| parse_bibtex_report(source, kwargs.recovery.policy())))
        .collect::<Vec<_>>();
    let mut keys = ListStringChunkedBuilder::new("keys".into(), reports.len(), reports.len());
    for report in &reports {
        match report.as_ref().and_then(|report| report.keys.as_ref()) {
            Some(values) => keys.append_values_iter(values.iter().map(String::as_str)),
            None => keys.append_null(),
        }
    }
    let fields = [
        BooleanChunked::from_iter_options(
            "ok".into(),
            reports.iter().map(|r| r.as_ref().map(|r| r.ok)),
        )
        .into_series(),
        UInt32Chunked::from_iter_options(
            "entry_count".into(),
            reports.iter().map(|r| {
                r.as_ref()
                    .and_then(|r| r.entry_count.and_then(|n| u32::try_from(n).ok()))
            }),
        )
        .into_series(),
        keys.finish().into_series(),
        diagnostic_lists_to_series(
            "diagnostics",
            reports
                .iter()
                .map(|r| r.as_ref().map(|r| r.diagnostics.as_slice())),
        )?,
    ];
    let result = StructChunked::from_series("parse_report".into(), reports.len(), fields.iter())?;
    with_struct_validity(result, reports.iter().map(Option::is_some))
}

pub(super) fn diagnostic_lists_to_series<'a>(
    name: &str,
    rows: impl Iterator<Item = Option<&'a [Diagnostic]>>,
) -> PolarsResult<Series> {
    let mut builder = AnonymousOwnedListBuilder::new(
        name.into(),
        rows.size_hint().0,
        Some(diagnostic_struct_dtype()),
    );
    for row in rows {
        let Some(diagnostics) = row else {
            builder.append_null();
            continue;
        };
        let span_fields = [
            UInt64Chunked::from_iter_options(
                "start".into(),
                diagnostics
                    .iter()
                    .map(|d| d.span.as_ref().map(|s| s.start as u64)),
            )
            .into_series(),
            UInt64Chunked::from_iter_options(
                "end".into(),
                diagnostics
                    .iter()
                    .map(|d| d.span.as_ref().map(|s| s.end as u64)),
            )
            .into_series(),
        ];
        let spans =
            StructChunked::from_series("span".into(), diagnostics.len(), span_fields.iter())?;
        let fields = [
            StringChunked::from_iter_values("code".into(), diagnostics.iter().map(|d| d.code))
                .into_series(),
            StringChunked::from_iter_values(
                "severity".into(),
                diagnostics.iter().map(|d| d.severity.as_str()),
            )
            .into_series(),
            StringChunked::from_iter_values(
                "action".into(),
                diagnostics.iter().map(|d| d.action.as_str()),
            )
            .into_series(),
            with_struct_validity(spans, diagnostics.iter().map(|d| d.span.is_some()))?,
            StringChunked::from_iter_options(
                "entry".into(),
                diagnostics.iter().map(|d| d.entry.as_deref()),
            )
            .into_series(),
            StringChunked::from_iter_options(
                "field".into(),
                diagnostics.iter().map(|d| d.field.as_deref()),
            )
            .into_series(),
            StringChunked::from_iter_values(
                "message".into(),
                diagnostics.iter().map(|d| d.message.as_str()),
            )
            .into_series(),
        ];
        builder.append_series(
            &StructChunked::from_series("diagnostic".into(), diagnostics.len(), fields.iter())?
                .into_series(),
        )?;
    }
    Ok(builder.finish().into_series())
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
