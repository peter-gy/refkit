use std::fmt;

use hayagriva::citationberg::{IndependentStyle, LocaleCode};
use hayagriva::{
    BibliographyDriver, BibliographyItem, BibliographyRequest, BufWriteFormat, CitationItem,
    CitationRequest, Library as HayLibrary,
};

use super::html::{render_child_html, render_children_html};
use super::text::elem_children_to_string;
use super::{RenderedOutput, bundled_locales};

pub(crate) fn render_bibliography(
    library: &HayLibrary,
    style: &IndependentStyle,
    locale: Option<&str>,
    all: bool,
) -> Result<RenderedOutput, String> {
    let locales = bundled_locales();
    let locale = locale.map(|code| LocaleCode(code.to_string()));
    let mut driver = BibliographyDriver::new();

    if all {
        for entry in library.iter() {
            driver.citation(CitationRequest::new(
                vec![CitationItem::with_entry(entry)],
                style,
                locale.clone(),
                locales,
                None,
            ));
        }
    }

    let rendered = driver.finish(BibliographyRequest::new(style, locale, locales));
    let Some(bibliography) = rendered.bibliography else {
        return Ok(RenderedOutput {
            text: String::new(),
            html: String::new(),
        });
    };
    let (text, html) = bibliography_to_text_html(&bibliography.items)?;
    Ok(RenderedOutput { text, html })
}

pub(crate) fn bibliography_to_text_html(
    items: &[BibliographyItem],
) -> Result<(String, String), String> {
    let mut text = String::with_capacity(items.len() * 224);
    let mut html = String::with_capacity(items.len() * 384);
    for item in items {
        if !text.is_empty() {
            text.push('\n');
        }
        write_bibliography_item_text(item, &mut text)?;
        render_bibliography_item_html(item, &mut html).map_err(|err| err.to_string())?;
    }
    Ok((text, html))
}

fn write_bibliography_item_text(
    item: &BibliographyItem,
    output: &mut String,
) -> Result<(), String> {
    let item_start = output.len();
    if let Some(first_field) = &item.first_field {
        first_field
            .write_buf(output, BufWriteFormat::Plain)
            .map_err(|err| err.to_string())?;
    }

    let content = elem_children_to_string(&item.content, BufWriteFormat::Plain)?;
    if output.len() > item_start && !content.is_empty() {
        output.push(' ');
    }
    output.push_str(&content);
    Ok(())
}

fn render_bibliography_item_html(item: &BibliographyItem, output: &mut String) -> fmt::Result {
    output.push_str("<div class=\"csl-entry\" data-key=\"");
    write_html_escaped(output, &item.key);
    output.push_str("\">");
    if let Some(first_field) = &item.first_field {
        output.push_str("<div class=\"csl-left-margin\">");
        render_child_html(first_field, output)?;
        output.push_str("</div><div class=\"csl-right-inline\">");
        render_children_html(&item.content, output)?;
        output.push_str("</div>");
    } else {
        render_children_html(&item.content, output)?;
    }
    output.push_str("</div>");
    Ok(())
}

fn write_html_escaped(output: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&#39;"),
            _ => output.push(ch),
        }
    }
}
