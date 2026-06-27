use polars::prelude::*;
use polars_core::chunked_array::builder::{AnonymousOwnedListBuilder, ListBuilderTrait};
use pyo3_polars::derive::polars_expr;
use pyo3_polars::export::polars_arrow::array::IntoBoxedArray;
use pyo3_polars::export::polars_arrow::bitmap::Bitmap;
use refkit_core::{
    CoreLibrary, PreparedStyle, RenderedOutput, render_library_bibliography,
    render_library_citation, render_library_citation_each, render_library_citation_group,
};

use super::RenderKwargs;
use super::broadcast::{
    broadcast_get, broadcast_len, load_style, parse_broadcast_library, parse_value_library_source,
};
use super::dtypes::{
    keys_output, rendered_list_output, rendered_output, rendered_struct_dtype, string_output,
};

#[derive(Clone, Copy)]
enum RenderedField {
    Text,
    Html,
}

#[polars_expr(output_type_func=string_output)]
fn cite(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_citation_field(inputs, kwargs, RenderedField::Text)
}

#[polars_expr(output_type_func=string_output)]
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

#[polars_expr(output_type_func=string_output)]
fn cite_group(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_citation_group_field(inputs, kwargs, RenderedField::Text)
}

#[polars_expr(output_type_func=string_output)]
fn cite_group_html(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_citation_group_field(inputs, kwargs, RenderedField::Html)
}

#[polars_expr(output_type_func=rendered_output)]
fn cite_group_rendered(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_citation_group_struct(inputs, kwargs, "cite_group_rendered")
}

#[polars_expr(output_type_func=string_output)]
fn full_bibliography_html(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_bibliography_field(inputs, kwargs, RenderedField::Html)
}

#[polars_expr(output_type_func=string_output)]
fn full_bibliography_text(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_bibliography_field(inputs, kwargs, RenderedField::Text)
}

#[polars_expr(output_type_func=rendered_output)]
fn full_bibliography_rendered(inputs: &[Series], kwargs: RenderKwargs) -> PolarsResult<Series> {
    render_bibliography_struct(inputs, kwargs, "full_bibliography_rendered")
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

fn rendered_field(rendered: RenderedOutput, field: RenderedField) -> String {
    match field {
        RenderedField::Text => rendered.text,
        RenderedField::Html => rendered.html,
    }
}
