use std::collections::HashMap;

use hayagriva::citationberg::{IndependentStyle, Locale as CslLocale, LocaleCode};
use hayagriva::{
    BibliographyDriver, BibliographyRequest, BufWriteFormat, CitationItem, CitationRequest,
    Library as HayLibrary, standalone_citation,
};

use crate::quoted;
use crate::style_analysis::can_fast_render_single_citations;

use super::html::elem_children_to_html;
use super::text::elem_children_to_string;
use super::{RenderedOutput, bundled_locales};

pub(crate) fn render_citation_group(
    library: &HayLibrary,
    keys: &[&str],
    style: &IndependentStyle,
    locale: Option<&str>,
) -> Result<RenderedOutput, String> {
    let locales = bundled_locales();
    let locale = locale.map(|code| LocaleCode(code.to_string()));
    let mut items = Vec::with_capacity(keys.len());
    for key in keys {
        let entry = library
            .get(key)
            .ok_or_else(|| format!("missing reference {}", quoted(key)))?;
        items.push(CitationItem::with_entry(entry));
    }
    let children = standalone_citation(CitationRequest::new(items, style, locale, locales, None));
    Ok(RenderedOutput {
        text: elem_children_to_string(&children, BufWriteFormat::Plain)?,
        html: elem_children_to_html(&children)?,
    })
}

pub(crate) fn render_citation(
    library: &HayLibrary,
    key: &str,
    style: &IndependentStyle,
    locale: Option<&str>,
) -> Result<RenderedOutput, String> {
    let locales = bundled_locales();
    let locale = locale.map(|code| LocaleCode(code.to_string()));
    if can_fast_render_single_citations(style) {
        return render_independent_citation_inner(library, key, style, locale, locales);
    }

    let entry = library
        .get(key)
        .ok_or_else(|| format!("missing reference {}", quoted(key)))?;
    let mut driver = BibliographyDriver::new();

    driver.citation(CitationRequest::new(
        vec![CitationItem::with_entry(entry)],
        style,
        locale.clone(),
        locales,
        None,
    ));

    let rendered = driver.finish(BibliographyRequest::new(style, locale, locales));
    let citation = rendered
        .citations
        .last()
        .ok_or_else(|| "citation renderer returned no citations".to_string())?;
    Ok(RenderedOutput {
        text: elem_children_to_string(&citation.citation, BufWriteFormat::Plain)?,
        html: elem_children_to_html(&citation.citation)?,
    })
}

pub(crate) fn render_citation_each(
    library: &HayLibrary,
    keys: &[&str],
    style: &IndependentStyle,
    locale: Option<&str>,
) -> Result<Vec<RenderedOutput>, String> {
    let locales = bundled_locales();
    let locale = locale.map(|code| LocaleCode(code.to_string()));
    if can_fast_render_single_citations(style)
        && let Some(rendered) =
            render_independent_citation_each(library, keys, style, locale.clone(), locales)?
    {
        return Ok(rendered);
    }

    let mut driver = BibliographyDriver::new();

    for key in keys {
        let entry = library
            .get(key)
            .ok_or_else(|| format!("missing reference {}", quoted(key)))?;
        driver.citation(CitationRequest::new(
            vec![CitationItem::with_entry(entry)],
            style,
            locale.clone(),
            locales,
            None,
        ));
    }

    let rendered = driver.finish(BibliographyRequest::new(style, locale, locales));
    if rendered.citations.len() != keys.len() {
        return Err("citation renderer returned an unexpected citation count".to_string());
    }

    rendered
        .citations
        .iter()
        .map(|citation| {
            Ok(RenderedOutput {
                text: elem_children_to_string(&citation.citation, BufWriteFormat::Plain)?,
                html: elem_children_to_html(&citation.citation)?,
            })
        })
        .collect()
}

fn render_independent_citation_each(
    library: &HayLibrary,
    keys: &[&str],
    style: &IndependentStyle,
    locale: Option<LocaleCode>,
    locales: &[CslLocale],
) -> Result<Option<Vec<RenderedOutput>>, String> {
    let mut key_by_text: HashMap<String, &str> = HashMap::new();
    let mut rendered = Vec::with_capacity(keys.len());

    for key in keys {
        let output =
            render_independent_citation_inner(library, key, style, locale.clone(), locales)?;
        if let Some(existing_key) = key_by_text.get(&output.text)
            && existing_key != key
        {
            return Ok(None);
        }
        key_by_text.insert(output.text.clone(), key);
        rendered.push(output);
    }

    Ok(Some(rendered))
}

fn render_independent_citation_inner(
    library: &HayLibrary,
    key: &str,
    style: &IndependentStyle,
    locale: Option<LocaleCode>,
    locales: &[CslLocale],
) -> Result<RenderedOutput, String> {
    let entry = library
        .get(key)
        .ok_or_else(|| format!("missing reference {}", quoted(key)))?;
    let children = standalone_citation(CitationRequest::new(
        vec![CitationItem::with_entry(entry)],
        style,
        locale,
        locales,
        None,
    ));
    Ok(RenderedOutput {
        text: elem_children_to_string(&children, BufWriteFormat::Plain)?,
        html: elem_children_to_html(&children)?,
    })
}
