use polars::prelude::*;
use polars_core::chunked_array::builder::{AnonymousOwnedListBuilder, ListBuilderTrait};
use pyo3_polars::derive::polars_expr;
use refkit_core::{
    Diagnostic, DocumentError, Library, LibraryError, PreparedStyle, RenderedOutput,
    render_library_bibliography, render_library_citation, render_library_citation_each,
    render_library_citation_group,
};

use super::RenderKwargs;
use super::broadcast::{
    broadcast_get, broadcast_len, input, load_style, parse_value_library_source,
};
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

struct RenderReportRow {
    result: Result<Vec<RenderedOutput>, (String, String)>,
    diagnostics: Vec<Diagnostic>,
}

struct RenderRow<'a> {
    result: Result<Vec<RenderedOutput>, RenderFailure<'a>>,
    diagnostics: &'a [Diagnostic],
}

enum RenderFailure<'a> {
    Parse(&'a LibraryError),
    Render(DocumentError),
}

impl RenderFailure<'_> {
    fn into_report(self) -> (String, String) {
        match self {
            Self::Parse(error) => ("parse_error".into(), error.to_string()),
            Self::Render(error) => {
                let code = if matches!(error, DocumentError::MissingReference(_)) {
                    "missing_key"
                } else {
                    "render_error"
                };
                (code.into(), error.to_string())
            }
        }
    }
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
    let mut rows = Vec::new();
    visit_render_rows(inputs, kwargs, operation, |row| {
        rows.push(row.map(|row| RenderReportRow {
            result: row.result.map_err(RenderFailure::into_report),
            diagnostics: row.diagnostics.to_vec(),
        }));
        Ok(())
    })?;
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
    if matches!(operation, Operation::Each) {
        render_list_values(inputs, kwargs, projection)
    } else {
        render_scalar_values(inputs, kwargs, operation, projection)
    }
}

fn render_list_values(
    inputs: &[Series],
    kwargs: RenderKwargs,
    projection: Projection,
) -> PolarsResult<Series> {
    let len = render_len(inputs, Operation::Each)?;
    if matches!(projection, Projection::Rendered) {
        let mut builder =
            AnonymousOwnedListBuilder::new("citations".into(), len, Some(rendered_struct_dtype()));
        visit_render_rows(inputs, kwargs, Operation::Each, |row| {
            match row.and_then(|row| row.result.ok()) {
                Some(values) => builder.append_series(&rendered_struct(
                    "citation",
                    &values.iter().map(Some).collect::<Vec<_>>(),
                )?)?,
                None => builder.append_null(),
            }
            Ok(())
        })?;
        return Ok(builder.finish().into_series());
    }
    let mut builder = ListStringChunkedBuilder::new("citations".into(), len, len);
    visit_render_rows(inputs, kwargs, Operation::Each, |row| {
        match row.and_then(|row| row.result.ok()) {
            Some(values) => {
                builder.append_values_iter(values.iter().map(|value| projected(value, projection)));
            }
            None => builder.append_null(),
        }
        Ok(())
    })?;
    Ok(builder.finish().into_series())
}

fn render_scalar_values(
    inputs: &[Series],
    kwargs: RenderKwargs,
    operation: Operation,
    projection: Projection,
) -> PolarsResult<Series> {
    let len = render_len(inputs, operation)?;
    if matches!(projection, Projection::Rendered) {
        let mut text = StringChunkedBuilder::new("text".into(), len);
        let mut html = StringChunkedBuilder::new("html".into(), len);
        let mut validity = Vec::with_capacity(len);
        visit_render_rows(inputs, kwargs, operation, |row| {
            let values = row.and_then(|row| row.result.ok());
            let value = values.as_ref().and_then(|values| values.first());
            text.append_option(value.map(|value| value.text.as_str()));
            html.append_option(value.map(|value| value.html.as_str()));
            validity.push(value.is_some());
            Ok(())
        })?;
        let fields = [text.finish().into_series(), html.finish().into_series()];
        let result = StructChunked::from_series("rendered".into(), len, fields.iter())?;
        return super::dtypes::with_struct_validity(result, validity.into_iter());
    }
    let mut builder = StringChunkedBuilder::new("rendered".into(), len);
    visit_render_rows(inputs, kwargs, operation, |row| {
        let values = row.and_then(|row| row.result.ok());
        let value = values.as_ref().and_then(|values| values.first());
        builder.append_option(value.map(|value| projected(value, projection)));
        Ok(())
    })?;
    Ok(builder.finish().into_series())
}

fn render_len(inputs: &[Series], operation: Operation) -> PolarsResult<usize> {
    let sources = input(inputs, 0)?.len();
    if matches!(operation, Operation::Bibliography) {
        Ok(sources)
    } else {
        broadcast_len(sources, input(inputs, 1)?.len(), "render")
    }
}

fn visit_render_rows(
    inputs: &[Series],
    kwargs: RenderKwargs,
    operation: Operation,
    mut visit: impl FnMut(Option<RenderRow<'_>>) -> PolarsResult<()>,
) -> PolarsResult<()> {
    let RenderKwargs {
        style,
        locale,
        recovery,
        ..
    } = kwargs;
    let sources = input(inputs, 0)?.str()?;
    let is_list = matches!(operation, Operation::Each | Operation::Group);
    let key_lists = if is_list {
        let lists = input(inputs, 1)?.list()?;
        if lists.inner_dtype() != &DataType::String {
            polars_bail!(InvalidOperation: "citation keys must have dtype List[String], got {}", input(inputs, 1)?.dtype());
        }
        Some(lists)
    } else {
        None
    };
    let keys = if matches!(operation, Operation::Single) {
        Some(input(inputs, 1)?.str()?)
    } else {
        None
    };
    let len = render_len(inputs, operation)?;
    let style = load_style(&style)?;
    let locale = Some(locale.as_str()).filter(|value| !value.is_empty());
    // Cache both success and failure for a literal source broadcast across rows.
    let cached = if sources.len() == 1 {
        sources
            .get(0)
            .map(|source| parse_value_library_source(source, recovery.policy()))
    } else {
        None
    };
    for index in 0..len {
        let Some(source) = broadcast_get(sources, index) else {
            visit(None)?;
            continue;
        };
        let selected = if let Some(keys) = keys {
            let Some(key) = broadcast_get(keys, index) else {
                visit(None)?;
                continue;
            };
            vec![key.to_owned()]
        } else if let Some(lists) = key_lists {
            let Some(keys) = lists.get_as_series(if lists.len() == 1 { 0 } else { index }) else {
                visit(None)?;
                continue;
            };
            let Some(keys) = keys
                .str()?
                .iter()
                .map(|key| key.map(str::to_owned))
                .collect::<Option<Vec<_>>>()
            else {
                visit(None)?;
                continue;
            };
            keys
        } else {
            Vec::new()
        };
        let parsed;
        let library = if let Some(parsed) = cached.as_ref() {
            parsed
        } else {
            parsed = parse_value_library_source(source, recovery.policy());
            &parsed
        };
        let row = match library {
            Ok(library) => RenderRow {
                diagnostics: library.diagnostics(),
                result: render_library(library, &selected, &style, locale, operation)
                    .map_err(RenderFailure::Render),
            },
            Err(error) => RenderRow {
                diagnostics: match error {
                    LibraryError::Biblatex(failure) => &failure.diagnostics,
                    _ => &[],
                },
                result: Err(RenderFailure::Parse(error)),
            },
        };
        visit(Some(row))?;
    }
    Ok(())
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
        Operation::Single => render_library_citation(
            library,
            keys.first().ok_or(DocumentError::EmptyCitation)?,
            style,
            locale,
        )
        .map(|value| vec![value]),
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

fn rendered_lists(rows: &[Option<RenderReportRow>]) -> PolarsResult<Series> {
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
