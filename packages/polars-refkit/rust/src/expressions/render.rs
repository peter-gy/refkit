use polars::prelude::*;
use polars_core::chunked_array::builder::{AnonymousOwnedListBuilder, ListBuilderTrait};
use pyo3_polars::derive::polars_expr;
use refkit_core::{
    Diagnostic, DocumentError, Library, LibraryError, PreparedStyle, RenderedOutput,
    render_library_bibliography, render_library_citation, render_library_citation_each,
    render_library_citation_group,
};

use super::RenderKwargs;
use super::broadcast::{broadcast_get, broadcast_len, load_style, parse_value_library_source};
use super::dtypes::{
    each_rendered_output, each_string_output, group_rendered_output, group_string_output,
    render_report_output, rendered_output, rendered_struct_dtype, string_output,
};
use super::parse::diagnostic_lists_to_series;

#[derive(Clone, Copy)]
enum Projection {
    Text,
    Html,
    Rendered,
}
#[derive(Clone, Copy)]
enum Operation {
    Single,
    Each,
    Group,
    Bibliography,
}

struct RenderRow {
    result: Result<Vec<RenderedOutput>, (String, String)>,
    diagnostics: Vec<Diagnostic>,
}

#[polars_expr(output_type_func=string_output)]
fn cite(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_values(inputs, kwargs, Operation::Single, Projection::Text)
}

#[polars_expr(output_type_func=string_output)]
fn cite_html(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_values(inputs, kwargs, Operation::Single, Projection::Html)
}

#[polars_expr(output_type_func=rendered_output)]
fn cite_rendered(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_values(inputs, kwargs, Operation::Single, Projection::Rendered)
}

#[polars_expr(output_type_func=each_string_output)]
fn cite_each(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_values(inputs, kwargs, Operation::Each, Projection::Text)
}

#[polars_expr(output_type_func=each_string_output)]
fn cite_each_html(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_values(inputs, kwargs, Operation::Each, Projection::Html)
}

#[polars_expr(output_type_func=each_rendered_output)]
fn cite_each_rendered(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_values(inputs, kwargs, Operation::Each, Projection::Rendered)
}

#[polars_expr(output_type_func=group_string_output)]
fn cite_group(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_values(inputs, kwargs, Operation::Group, Projection::Text)
}

#[polars_expr(output_type_func=group_string_output)]
fn cite_group_html(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_values(inputs, kwargs, Operation::Group, Projection::Html)
}

#[polars_expr(output_type_func=group_rendered_output)]
fn cite_group_rendered(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_values(inputs, kwargs, Operation::Group, Projection::Rendered)
}

#[polars_expr(output_type_func=string_output)]
fn full_bibliography_html(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_values(inputs, kwargs, Operation::Bibliography, Projection::Html)
}

#[polars_expr(output_type_func=string_output)]
fn full_bibliography_text(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_values(inputs, kwargs, Operation::Bibliography, Projection::Text)
}

#[polars_expr(output_type_func=rendered_output)]
fn full_bibliography_rendered(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_values(
        inputs,
        kwargs,
        Operation::Bibliography,
        Projection::Rendered,
    )
}

#[polars_expr(output_type_func=render_report_output)]
fn render_report(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    let operation = if kwargs.grouped {
        Operation::Group
    } else {
        Operation::Each
    };
    let rows = render_rows(inputs, &kwargs, operation)?;
    let ok = BooleanChunked::from_iter_options(
        "ok".into(),
        rows.iter()
            .map(|row| row.as_ref().map(|r| r.result.is_ok())),
    )
    .into_series();
    let citations = rendered_lists(&rows)?;
    let diagnostics = diagnostic_lists_to_series(
        "diagnostics",
        rows.iter()
            .map(|row| row.as_ref().map(|r| r.diagnostics.as_slice())),
    )?;
    let error_code = StringChunked::from_iter_options(
        "error_code".into(),
        rows.iter().map(|row| {
            row.as_ref()
                .and_then(|r| r.result.as_ref().err().map(|e| e.0.as_str()))
        }),
    )
    .into_series();
    let error = StringChunked::from_iter_options(
        "error".into(),
        rows.iter().map(|row| {
            row.as_ref()
                .and_then(|r| r.result.as_ref().err().map(|e| e.1.as_str()))
        }),
    )
    .into_series();
    let fields = [
        ok,
        citations.with_name("citations".into()),
        diagnostics,
        error_code,
        error,
    ];
    let result = StructChunked::from_series("render_report".into(), rows.len(), fields.iter())?;
    super::dtypes::with_struct_validity(result, rows.iter().map(Option::is_some))
}

fn render_values(
    inputs: &[Series],
    kwargs: RenderKwargs,
    operation: Operation,
    projection: Projection,
) -> PolarsResult<Series> {
    let rows = render_rows(inputs, &kwargs, operation)?;
    if matches!(operation, Operation::Each) {
        if matches!(projection, Projection::Rendered) {
            return rendered_lists(&rows);
        }
        let mut builder = ListStringChunkedBuilder::new("citations".into(), rows.len(), rows.len());
        for row in &rows {
            match row.as_ref().and_then(|r| r.result.as_ref().ok()) {
                Some(values) => builder
                    .append_values_iter(values.iter().map(|value| projected(value, projection))),
                None => builder.append_null(),
            }
        }
        return Ok(builder.finish().into_series());
    }
    let values = rows
        .iter()
        .map(|row| {
            row.as_ref()
                .and_then(|r| r.result.as_ref().ok())
                .and_then(|values| values.first())
        })
        .collect::<Vec<_>>();
    if matches!(projection, Projection::Rendered) {
        return rendered_struct("rendered", &values);
    }
    Ok(StringChunked::from_iter_options(
        "rendered".into(),
        values
            .iter()
            .map(|value| value.map(|value| projected(value, projection))),
    )
    .into_series())
}

fn render_rows(
    inputs: &[Series],
    kwargs: &RenderKwargs,
    operation: Operation,
) -> PolarsResult<Vec<Option<RenderRow>>> {
    let sources = inputs[0].str()?;
    let is_list = matches!(operation, Operation::Each | Operation::Group);
    let key_lists = if is_list {
        let lists = inputs[1].list()?;
        if lists.inner_dtype() != &DataType::String {
            polars_bail!(InvalidOperation: "citation keys must have dtype List[String], got {}", inputs[1].dtype());
        }
        Some(lists)
    } else {
        None
    };
    let keys = if matches!(operation, Operation::Single) {
        Some(inputs[1].str()?)
    } else {
        None
    };
    let len = if matches!(operation, Operation::Bibliography) {
        sources.len()
    } else {
        broadcast_len(sources.len(), inputs[1].len(), "render")?
    };
    let style = load_style(&kwargs.style)?;
    let locale = Some(kwargs.locale.as_str()).filter(|value| !value.is_empty());
    // Cache both success and failure for a literal source broadcast across rows.
    let cached = if sources.len() == 1 {
        sources
            .get(0)
            .map(|source| parse_value_library_source(source, kwargs.recovery.policy()))
    } else {
        None
    };
    let mut rows = Vec::with_capacity(len);
    for index in 0..len {
        let Some(source) = broadcast_get(sources, index) else {
            rows.push(None);
            continue;
        };
        let selected = if let Some(keys) = keys {
            let Some(key) = broadcast_get(keys, index) else {
                rows.push(None);
                continue;
            };
            vec![key.to_owned()]
        } else if let Some(lists) = key_lists {
            let Some(keys) = lists.get_as_series(if lists.len() == 1 { 0 } else { index }) else {
                rows.push(None);
                continue;
            };
            let Some(keys) = keys
                .str()?
                .into_iter()
                .map(|key| key.map(str::to_owned))
                .collect::<Option<Vec<_>>>()
            else {
                rows.push(None);
                continue;
            };
            keys
        } else {
            Vec::new()
        };
        let parsed;
        let library = match cached.as_ref() {
            Some(parsed) => parsed,
            None => {
                parsed = parse_value_library_source(source, kwargs.recovery.policy());
                &parsed
            }
        };
        let row = match library {
            Ok(library) => RenderRow {
                diagnostics: library.diagnostics().to_vec(),
                result: render_library(library, &selected, &style, locale, operation).map_err(
                    |error| {
                        let code = if matches!(error, DocumentError::MissingReference(_)) {
                            "missing_key"
                        } else {
                            "render_error"
                        };
                        (code.to_owned(), error.to_string())
                    },
                ),
            },
            Err(error) => RenderRow {
                diagnostics: match error {
                    LibraryError::Biblatex(failure) => failure.diagnostics.clone(),
                    _ => Vec::new(),
                },
                result: Err(("parse_error".into(), error.to_string())),
            },
        };
        rows.push(Some(row));
    }
    Ok(rows)
}

fn render_library(
    library: &Library,
    keys: &[String],
    style: &PreparedStyle,
    locale: Option<&str>,
    operation: Operation,
) -> Result<Vec<RenderedOutput>, DocumentError> {
    let keys = keys.iter().map(String::as_str).collect::<Vec<_>>();
    match operation {
        Operation::Single => {
            render_library_citation(library, keys[0], style, locale).map(|value| vec![value])
        }
        Operation::Each => render_library_citation_each(library, &keys, style, locale),
        Operation::Group => {
            render_library_citation_group(library, &keys, style, locale).map(|value| vec![value])
        }
        Operation::Bibliography => {
            render_library_bibliography(library, style, locale).map(|value| vec![value])
        }
    }
}

fn projected(value: &RenderedOutput, projection: Projection) -> &str {
    match projection {
        Projection::Html => &value.html,
        _ => &value.text,
    }
}

fn rendered_lists(rows: &[Option<RenderRow>]) -> PolarsResult<Series> {
    let mut builder = AnonymousOwnedListBuilder::new(
        "citations".into(),
        rows.len(),
        Some(rendered_struct_dtype()),
    );
    for row in rows {
        match row.as_ref().and_then(|row| row.result.as_ref().ok()) {
            Some(values) => builder.append_series(&rendered_struct(
                "citation",
                &values.iter().map(Some).collect::<Vec<_>>(),
            )?)?,
            None => builder.append_null(),
        }
    }
    Ok(builder.finish().into_series())
}

fn rendered_struct(name: &str, values: &[Option<&RenderedOutput>]) -> PolarsResult<Series> {
    let fields = [
        StringChunked::from_iter_options(
            "text".into(),
            values.iter().map(|value| value.map(|v| v.text.as_str())),
        )
        .into_series(),
        StringChunked::from_iter_options(
            "html".into(),
            values.iter().map(|value| value.map(|v| v.html.as_str())),
        )
        .into_series(),
    ];
    let result = StructChunked::from_series(name.into(), values.len(), fields.iter())?;
    super::dtypes::with_struct_validity(result, values.iter().map(Option::is_some))
}
