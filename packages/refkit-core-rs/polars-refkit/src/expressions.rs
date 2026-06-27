use polars::prelude::*;
use polars_core::chunked_array::builder::{AnonymousOwnedListBuilder, ListBuilderTrait};
use pyo3_polars::derive::polars_expr;
use pyo3_polars::export::polars_arrow::array::IntoBoxedArray;
use pyo3_polars::export::polars_arrow::bitmap::Bitmap;
use refkit_core::{
    CoreLibrary, NormalizedEntry, NormalizedValue, PreparedStyle, ProjectField, RenderedOutput,
    load_prepared_style, parse_bibtex_report_source, parse_project_field,
    render_library_bibliography, render_library_citation, render_library_citation_each,
    render_library_citation_group,
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct ParseKwargs {
    strict: bool,
}

#[derive(Debug, Deserialize)]
struct EntriesKwargs {
    strict: bool,
    fields: Vec<String>,
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

fn entries_output(input_fields: &[Field], kwargs: EntriesKwargs) -> PolarsResult<Field> {
    let field = input_fields[0].clone();
    let field_names = kwargs.fields.iter().map(String::as_str).collect::<Vec<_>>();
    Ok(Field::new(
        field.name,
        DataType::List(Box::new(entry_struct_dtype(&field_names))),
    ))
}

fn rendered_output(input_fields: &[Field]) -> PolarsResult<Field> {
    let field = input_fields[0].clone();
    Ok(Field::new(field.name, rendered_struct_dtype()))
}

fn rendered_list_output(input_fields: &[Field]) -> PolarsResult<Field> {
    let field = input_fields[0].clone();
    Ok(Field::new(
        field.name,
        DataType::List(Box::new(rendered_struct_dtype())),
    ))
}

fn parse_report_output(input_fields: &[Field]) -> PolarsResult<Field> {
    let field = input_fields[0].clone();
    Ok(Field::new(field.name, parse_report_struct_dtype()))
}

#[polars_expr(output_type=String)]
fn cite(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_citation_field(inputs, kwargs, RenderedField::Text)
}

#[polars_expr(output_type=String)]
fn cite_html(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_citation_field(inputs, kwargs, RenderedField::Html)
}

#[polars_expr(output_type_func=rendered_output)]
fn cite_rendered(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_citation_struct(inputs, kwargs, "cite_rendered")
}

#[polars_expr(output_type_func=keys_output)]
fn cite_each(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_citation_each_field(inputs, kwargs, RenderedField::Text)
}

#[polars_expr(output_type_func=keys_output)]
fn cite_each_html(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_citation_each_field(inputs, kwargs, RenderedField::Html)
}

#[polars_expr(output_type_func=rendered_list_output)]
fn cite_each_rendered(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_citation_each_struct(inputs, kwargs, "cite_each_rendered")
}

#[polars_expr(output_type=String)]
fn cite_group(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_citation_group_field(inputs, kwargs, RenderedField::Text)
}

#[polars_expr(output_type=String)]
fn cite_group_html(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_citation_group_field(inputs, kwargs, RenderedField::Html)
}

#[polars_expr(output_type_func=rendered_output)]
fn cite_group_rendered(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_citation_group_struct(inputs, kwargs, "cite_group_rendered")
}

fn render_citation_field(
    inputs: &[Series],
    kwargs: RenderKwargs,
    field: RenderedField,
) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let keys = inputs[1].str()?;
    let len = broadcast_len(bibtex.len(), keys.len(), "cite")?;
    let style = load_style(&kwargs.style)?;
    let locale = Some(kwargs.locale.as_str()).filter(|value| !value.is_empty());

    let output = match parse_broadcast_library(bibtex, kwargs.strict) {
        Some(library) => (0..len)
            .map(|index| {
                let key = broadcast_get(keys, index)?;
                render_library_citation(&library, key, style.as_ref(), locale)
                    .ok()
                    .map(|rendered| rendered_field(rendered, field))
            })
            .collect::<StringChunked>(),
        None => (0..len)
            .map(|index| {
                let source = broadcast_get(bibtex, index)?;
                let key = broadcast_get(keys, index)?;
                let library = parse_value_library_source(source, kwargs.strict).ok()?;
                render_library_citation(&library, key, style.as_ref(), locale)
                    .ok()
                    .map(|rendered| rendered_field(rendered, field))
            })
            .collect::<StringChunked>(),
    };
    Ok(output.into_series())
}

fn render_citation_each_field(
    inputs: &[Series],
    kwargs: RenderKwargs,
    field: RenderedField,
) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let key_lists = inputs[1].list()?;
    let len = broadcast_len(bibtex.len(), key_lists.len(), "cite_each")?;
    let style = load_style(&kwargs.style)?;
    let locale = Some(kwargs.locale.as_str()).filter(|value| !value.is_empty());
    let mut builder = ListStringChunkedBuilder::new("cite_each".into(), len, len * 2);

    match parse_broadcast_library(bibtex, kwargs.strict) {
        Some(library) => {
            for index in 0..len {
                append_citation_each_field(
                    &mut builder,
                    &library,
                    key_lists,
                    index,
                    &style,
                    locale,
                    field,
                )?;
            }
        }
        None => {
            for index in 0..len {
                let Some(source) = broadcast_get(bibtex, index) else {
                    builder.append_null();
                    continue;
                };
                match parse_value_library_source(source, kwargs.strict) {
                    Ok(library) => append_citation_each_field(
                        &mut builder,
                        &library,
                        key_lists,
                        index,
                        &style,
                        locale,
                        field,
                    )?,
                    Err(_) => builder.append_null(),
                }
            }
        }
    }

    Ok(builder.finish().into_series())
}

fn render_citation_group_field(
    inputs: &[Series],
    kwargs: RenderKwargs,
    field: RenderedField,
) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let key_lists = inputs[1].list()?;
    let len = broadcast_len(bibtex.len(), key_lists.len(), "cite_group")?;
    let style = load_style(&kwargs.style)?;
    let locale = Some(kwargs.locale.as_str()).filter(|value| !value.is_empty());

    let output = match parse_broadcast_library(bibtex, kwargs.strict) {
        Some(library) => (0..len)
            .map(|index| {
                render_citation_group_at(&library, key_lists, index, &style, locale)
                    .ok()
                    .flatten()
                    .map(|value| rendered_field(value, field))
            })
            .collect::<StringChunked>(),
        None => (0..len)
            .map(|index| {
                let source = broadcast_get(bibtex, index)?;
                let library = parse_value_library_source(source, kwargs.strict).ok()?;
                render_citation_group_at(&library, key_lists, index, &style, locale)
                    .ok()
                    .flatten()
                    .map(|value| rendered_field(value, field))
            })
            .collect::<StringChunked>(),
    };
    Ok(output.into_series())
}

#[polars_expr(output_type=String)]
fn full_bibliography_html(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_bibliography_field(inputs, kwargs, RenderedField::Html)
}

#[polars_expr(output_type=String)]
fn full_bibliography_text(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_bibliography_field(inputs, kwargs, RenderedField::Text)
}

#[polars_expr(output_type_func=rendered_output)]
fn full_bibliography_rendered(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_bibliography_struct(inputs, kwargs, "full_bibliography_rendered")
}

fn render_bibliography_field(
    inputs: &[Series],
    kwargs: RenderKwargs,
    field: RenderedField,
) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let style = load_style(&kwargs.style)?;
    let locale = Some(kwargs.locale.as_str()).filter(|value| !value.is_empty());

    let output = bibtex
        .iter()
        .map(|value| {
            let source = value?;
            let library = parse_value_library_source(source, kwargs.strict).ok()?;
            render_library_bibliography(&library, style.as_ref(), locale, kwargs.all)
                .ok()
                .map(|rendered| rendered_field(rendered, field))
        })
        .collect::<StringChunked>();
    Ok(output.into_series())
}

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

#[polars_expr(output_type_func_with_kwargs=entries_output)]
fn entries(inputs: &[Series], kwargs: EntriesKwargs) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let fields = parse_project_fields(&kwargs.fields).map_err(compute_error)?;
    let field_names = kwargs.fields.iter().map(String::as_str).collect::<Vec<_>>();
    let mut builder = AnonymousOwnedListBuilder::new(
        "entries".into(),
        bibtex.len(),
        Some(entry_struct_dtype(&field_names)),
    );

    for value in bibtex.iter() {
        let Some(source) = value else {
            builder.append_null();
            continue;
        };
        match parse_value_library_source(source, kwargs.strict) {
            Ok(library) => {
                let records = library
                    .project_records(&fields, None)
                    .map_err(compute_error)?;
                let entries = entry_records_to_struct_series(records, &field_names)?;
                builder.append_series(&entries)?;
            }
            Err(_) => builder.append_null(),
        }
    }

    Ok(builder.finish().into_series())
}

#[polars_expr(output_type=String)]
fn to_hayagriva_json(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
    bibtex_to_normalized_json(inputs, kwargs)
}

fn bibtex_to_normalized_json(inputs: &[Series], kwargs: ParseKwargs) -> PolarsResult<Series> {
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

#[derive(Clone, Copy)]
enum RenderedField {
    Text,
    Html,
}

fn render_citation_struct(
    inputs: &[Series],
    kwargs: RenderKwargs,
    name: &str,
) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let keys = inputs[1].str()?;
    let len = broadcast_len(bibtex.len(), keys.len(), name)?;
    let style = load_style(&kwargs.style)?;
    let locale = Some(kwargs.locale.as_str()).filter(|value| !value.is_empty());

    let rendered = match parse_broadcast_library(bibtex, kwargs.strict) {
        Some(library) => (0..len)
            .map(|index| {
                let key = broadcast_get(keys, index)?;
                render_library_citation(&library, key, style.as_ref(), locale).ok()
            })
            .collect::<Vec<_>>(),
        None => (0..len)
            .map(|index| {
                let source = broadcast_get(bibtex, index)?;
                let key = broadcast_get(keys, index)?;
                let library = parse_value_library_source(source, kwargs.strict).ok()?;
                render_library_citation(&library, key, style.as_ref(), locale).ok()
            })
            .collect::<Vec<_>>(),
    };
    rendered_outputs_to_struct_series(name, &rendered)
}

fn render_citation_each_struct(
    inputs: &[Series],
    kwargs: RenderKwargs,
    name: &str,
) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let key_lists = inputs[1].list()?;
    let len = broadcast_len(bibtex.len(), key_lists.len(), name)?;
    let style = load_style(&kwargs.style)?;
    let locale = Some(kwargs.locale.as_str()).filter(|value| !value.is_empty());
    let mut builder =
        AnonymousOwnedListBuilder::new(name.into(), len, Some(rendered_struct_dtype()));

    match parse_broadcast_library(bibtex, kwargs.strict) {
        Some(library) => {
            for index in 0..len {
                append_citation_each_struct(
                    &mut builder,
                    &library,
                    key_lists,
                    index,
                    &style,
                    locale,
                )?;
            }
        }
        None => {
            for index in 0..len {
                let Some(source) = broadcast_get(bibtex, index) else {
                    builder.append_null();
                    continue;
                };
                match parse_value_library_source(source, kwargs.strict) {
                    Ok(library) => append_citation_each_struct(
                        &mut builder,
                        &library,
                        key_lists,
                        index,
                        &style,
                        locale,
                    )?,
                    Err(_) => builder.append_null(),
                }
            }
        }
    }

    Ok(builder.finish().into_series())
}

fn render_citation_group_struct(
    inputs: &[Series],
    kwargs: RenderKwargs,
    name: &str,
) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let key_lists = inputs[1].list()?;
    let len = broadcast_len(bibtex.len(), key_lists.len(), name)?;
    let style = load_style(&kwargs.style)?;
    let locale = Some(kwargs.locale.as_str()).filter(|value| !value.is_empty());

    let rendered = match parse_broadcast_library(bibtex, kwargs.strict) {
        Some(library) => (0..len)
            .map(|index| {
                render_citation_group_at(&library, key_lists, index, &style, locale).ok()?
            })
            .collect::<Vec<_>>(),
        None => (0..len)
            .map(|index| {
                let source = broadcast_get(bibtex, index)?;
                let library = parse_value_library_source(source, kwargs.strict).ok()?;
                render_citation_group_at(&library, key_lists, index, &style, locale).ok()?
            })
            .collect::<Vec<_>>(),
    };
    rendered_outputs_to_struct_series(name, &rendered)
}

fn render_bibliography_struct(
    inputs: &[Series],
    kwargs: RenderKwargs,
    name: &str,
) -> PolarsResult<Series> {
    let bibtex = inputs[0].str()?;
    let style = load_style(&kwargs.style)?;
    let locale = Some(kwargs.locale.as_str()).filter(|value| !value.is_empty());

    let rendered = bibtex
        .iter()
        .map(|value| {
            let source = value?;
            let library = parse_value_library_source(source, kwargs.strict).ok()?;
            render_library_bibliography(&library, style.as_ref(), locale, kwargs.all).ok()
        })
        .collect::<Vec<_>>();
    rendered_outputs_to_struct_series(name, &rendered)
}

fn append_citation_each_field(
    builder: &mut ListStringChunkedBuilder,
    library: &CoreLibrary,
    key_lists: &ListChunked,
    index: usize,
    style: &PreparedStyle,
    locale: Option<&str>,
    field: RenderedField,
) -> PolarsResult<()> {
    let Some(keys) = citation_keys_at(key_lists, index)? else {
        builder.append_null();
        return Ok(());
    };

    let key_refs = keys.iter().map(String::as_str).collect::<Vec<_>>();
    match render_library_citation_each(library, &key_refs, style, locale) {
        Ok(rendered) => {
            let values = rendered
                .into_iter()
                .map(|value| rendered_field(value, field))
                .collect::<Vec<_>>();
            builder.append_values_iter(values.iter().map(String::as_str));
        }
        Err(_) => builder.append_null(),
    }
    Ok(())
}

fn append_citation_each_struct(
    builder: &mut AnonymousOwnedListBuilder,
    library: &CoreLibrary,
    key_lists: &ListChunked,
    index: usize,
    style: &PreparedStyle,
    locale: Option<&str>,
) -> PolarsResult<()> {
    let Some(keys) = citation_keys_at(key_lists, index)? else {
        builder.append_null();
        return Ok(());
    };

    let key_refs = keys.iter().map(String::as_str).collect::<Vec<_>>();
    match render_library_citation_each(library, &key_refs, style, locale) {
        Ok(rendered) => {
            let rendered = rendered.into_iter().map(Some).collect::<Vec<_>>();
            let entries = rendered_outputs_to_struct_series("citation", &rendered)?;
            builder.append_series(&entries)?;
        }
        Err(_) => builder.append_null(),
    }
    Ok(())
}

fn render_citation_group_at(
    library: &CoreLibrary,
    key_lists: &ListChunked,
    index: usize,
    style: &PreparedStyle,
    locale: Option<&str>,
) -> PolarsResult<Option<RenderedOutput>> {
    let Some(keys) = citation_keys_at(key_lists, index)? else {
        return Ok(None);
    };
    let key_refs = keys.iter().map(String::as_str).collect::<Vec<_>>();
    Ok(render_library_citation_group(library, &key_refs, style, locale).ok())
}

fn citation_keys_at(key_lists: &ListChunked, index: usize) -> PolarsResult<Option<Vec<String>>> {
    let Some(keys) = key_lists.get_as_series(if key_lists.len() == 1 { 0 } else { index }) else {
        return Ok(None);
    };
    Ok(keys
        .str()?
        .iter()
        .map(|value| value.map(str::to_string))
        .collect())
}

fn rendered_outputs_to_struct_series(
    name: &str,
    rendered: &[Option<RenderedOutput>],
) -> PolarsResult<Series> {
    let text = StringChunked::from_iter_options(
        "text".into(),
        rendered
            .iter()
            .map(|value| value.as_ref().map(|output| output.text.as_str())),
    )
    .into_series();
    let html = StringChunked::from_iter_options(
        "html".into(),
        rendered
            .iter()
            .map(|value| value.as_ref().map(|output| output.html.as_str())),
    )
    .into_series();
    let fields = [text, html];
    let chunked = StructChunked::from_series(name.into(), rendered.len(), fields.iter())?;
    if rendered.iter().all(Option::is_some) {
        return Ok(chunked.into_series());
    }

    let validity = Bitmap::from_iter(rendered.iter().map(Option::is_some));
    let chunks = chunked
        .downcast_iter()
        .map(|array| {
            array
                .clone()
                .with_validity(Some(validity.clone()))
                .into_boxed()
        })
        .collect::<Vec<_>>();
    Ok(unsafe {
        StructChunked::from_chunks_and_dtype(
            chunked.name().clone(),
            chunks,
            chunked.dtype().clone(),
        )
    }
    .into_series())
}

fn entry_records_to_struct_series(
    records: Vec<Vec<Option<String>>>,
    field_names: &[&str],
) -> PolarsResult<Series> {
    let fields = field_names
        .iter()
        .enumerate()
        .map(|(field_index, field_name)| {
            StringChunked::from_iter_options(
                (*field_name).into(),
                records
                    .iter()
                    .map(move |record| record[field_index].as_deref()),
            )
            .into_series()
        })
        .collect::<Vec<_>>();
    StructChunked::from_series("entry".into(), records.len(), fields.iter())
        .map(|entries| entries.into_series())
}

fn entry_struct_dtype(field_names: &[&str]) -> DataType {
    DataType::Struct(
        field_names
            .iter()
            .map(|field| Field::new((*field).into(), DataType::String))
            .collect(),
    )
}

fn rendered_struct_dtype() -> DataType {
    DataType::Struct(vec![
        Field::new("text".into(), DataType::String),
        Field::new("html".into(), DataType::String),
    ])
}

fn parse_report_struct_dtype() -> DataType {
    DataType::Struct(vec![
        Field::new("ok".into(), DataType::Boolean),
        Field::new("entry_count".into(), DataType::UInt32),
        Field::new("keys".into(), DataType::List(Box::new(DataType::String))),
        Field::new(
            "diagnostics".into(),
            DataType::List(Box::new(DataType::String)),
        ),
    ])
}

fn rendered_field(rendered: RenderedOutput, field: RenderedField) -> String {
    match field {
        RenderedField::Text => rendered.text,
        RenderedField::Html => rendered.html,
    }
}

fn parse_value_library_source(source: &str, strict: bool) -> Result<CoreLibrary, String> {
    CoreLibrary::parse_bibtex(source, strict)
}

fn parse_project_fields(fields: &[String]) -> Result<Vec<ProjectField>, String> {
    fields
        .iter()
        .map(|field| parse_project_field(field))
        .collect()
}

fn parse_broadcast_library(bibtex: &StringChunked, strict: bool) -> Option<CoreLibrary> {
    if bibtex.len() != 1 {
        return None;
    }
    parse_value_library_source(bibtex.get(0)?, strict).ok()
}

fn load_style(name: &str) -> PolarsResult<std::sync::Arc<PreparedStyle>> {
    load_prepared_style(name).map_err(|err| compute_error(err.to_string()))
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
