use polars::prelude::*;
use pyo3_polars::derive::polars_expr;
use refkit_core::{
    library_to_normalized_json, load_independent_style, parse_library_source, render_bibliography,
    render_citation,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ParseKwargs {
    strict: bool,
}

#[derive(Debug, Deserialize)]
struct RenderKwargs {
    style: String,
    locale: String,
    strict: bool,
    all: bool,
}

fn keys_output(input_fields: &[Field]) -> PolarsResult<Field> {
    let field = input_fields[0].clone();
    Ok(Field::new(
        field.name,
        DataType::List(Box::new(DataType::String)),
    ))
}

#[polars_expr(output_type=String)]
fn cite_bibtex(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let keys = inputs[1].str()?;
    let len = broadcast_len(bibtex.len(), keys.len(), "cite_bibtex")?;
    let style = load_independent_style(&kwargs.style).map_err(compute_error)?;
    let locale = Some(kwargs.locale.as_str()).filter(|value| !value.is_empty());

    let output = (0..len)
        .map(|index| {
            let source = broadcast_get(bibtex, index)?;
            let key = broadcast_get(keys, index)?;
            let library = parse_library_source(source, "bibtex", kwargs.strict, false).ok()?;
            render_citation(&library.inner, key, &style, locale)
                .ok()
                .map(|rendered| rendered.text)
        })
        .collect::<StringChunked>();
    Ok(output.into_series())
}

#[polars_expr(output_type=String)]
fn bibliography_bibtex(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let style = load_independent_style(&kwargs.style).map_err(compute_error)?;
    let locale = Some(kwargs.locale.as_str()).filter(|value| !value.is_empty());

    let output = bibtex
        .iter()
        .map(|value| {
            let source = value?;
            let library = parse_library_source(source, "bibtex", kwargs.strict, false).ok()?;
            render_bibliography(&library.inner, &style, locale, kwargs.all)
                .ok()
                .map(|rendered| rendered.html)
        })
        .collect::<StringChunked>();
    Ok(output.into_series())
}

#[polars_expr(output_type=UInt32)]
fn bibtex_entry_count(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let output = bibtex
        .iter()
        .map(|value| {
            value.and_then(|source| {
                parse_library_source(source, "bibtex", kwargs.strict, false)
                    .ok()
                    .and_then(|library| u32::try_from(library.inner.len()).ok())
            })
        })
        .collect::<UInt32Chunked>();
    Ok(output.into_series())
}

#[polars_expr(output_type_func=keys_output)]
fn bibtex_keys(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let mut builder =
        ListStringChunkedBuilder::new("bibtex_keys".into(), bibtex.len(), bibtex.len() * 2);

    for value in bibtex.iter() {
        let Some(source) = value else {
            builder.append_null();
            continue;
        };
        match parse_library_source(source, "bibtex", kwargs.strict, false) {
            Ok(library) => {
                let keys = library.inner.keys().collect::<Vec<_>>();
                builder.append_values_iter(keys.into_iter());
            }
            Err(_) => builder.append_null(),
        }
    }

    Ok(builder.finish().into_series())
}

#[polars_expr(output_type_func=keys_output)]
fn bibtex_diagnostics(inputs: &[Series]) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let mut builder =
        ListStringChunkedBuilder::new("bibtex_diagnostics".into(), bibtex.len(), bibtex.len());

    for value in bibtex.iter() {
        let Some(source) = value else {
            builder.append_null();
            continue;
        };
        match parse_library_source(source, "bibtex", false, true) {
            Ok(library) => {
                let diagnostics = library.diagnostics.iter().map(String::as_str);
                builder.append_values_iter(diagnostics);
            }
            Err(err) => builder.append_values_iter(std::iter::once(err.as_str())),
        }
    }

    Ok(builder.finish().into_series())
}

#[polars_expr(output_type=String)]
fn bibtex_to_csl_json(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let output = bibtex
        .iter()
        .map(|value| {
            let source = value?;
            let library = parse_library_source(source, "bibtex", kwargs.strict, false).ok()?;
            library_to_normalized_json(&library.inner).ok()
        })
        .collect::<StringChunked>();
    Ok(output.into_series())
}

fn broadcast_len(left: usize, right: usize, operation: &str) -> PolarsResult<usize> {
    if left == right {
        return Ok(left);
    }
    if left == 1 {
        return Ok(right);
    }
    if right == 1 {
        return Ok(left);
    }
    polars_bail!(
        ComputeError:
        "{} input lengths must match or broadcast one side, got bibtex={} and key={}",
        operation,
        left,
        right
    )
}

fn broadcast_get(values: &StringChunked, index: usize) -> Option<&str> {
    values.get(if values.len() == 1 { 0 } else { index })
}

fn compute_error(err: String) -> PolarsError {
    PolarsError::ComputeError(err.into())
}
