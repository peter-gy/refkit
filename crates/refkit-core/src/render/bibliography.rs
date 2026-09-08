use std::fmt::{self, Write as _};

use hayagriva::{BibliographyItem, BufWriteFormat, RenderedBibliography};

use super::html::{render_child_html, render_children_html, write_html_escaped};

pub(crate) fn bibliography_to_text_html(
    bibliography: &RenderedBibliography,
) -> Result<(String, String), String> {
    let items = &bibliography.items;
    let mut text = String::with_capacity(items.len() * 224);
    let mut html = String::with_capacity(items.len() * 384);
    if !items.is_empty() {
        write!(
            html,
            "<div class=\"csl-bib-body\" style=\"line-height:{};",
            bibliography.line_spacing
        )
        .map_err(|err| err.to_string())?;
        if let Some(align) = bibliography.second_field_align {
            let columns = match align {
                hayagriva::citationberg::SecondFieldAlign::Flush => "max-content minmax(0,1fr)",
                hayagriva::citationberg::SecondFieldAlign::Margin => "0 minmax(0,1fr)",
            };
            write!(
                html,
                "display:grid;grid-template-columns:{columns};column-gap:0.5em;row-gap:{}em;",
                bibliography.entry_spacing
            )
            .map_err(|err| err.to_string())?;
        }
        html.push_str("\">");
    }
    for (index, item) in items.iter().enumerate() {
        if !text.is_empty() {
            text.push('\n');
        }
        write_bibliography_item_text(item, &mut text)?;
        render_bibliography_item_html(item, bibliography, index, &mut html)
            .map_err(|err| err.to_string())?;
    }
    if !items.is_empty() {
        html.push_str("</div>");
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

    let label_end = output.len();
    if label_end > item_start {
        output.push(' ');
    }
    let content_start = output.len();
    item.content
        .write_buf(output, BufWriteFormat::Plain)
        .map_err(|err| err.to_string())?;
    if output.len() == content_start {
        output.truncate(label_end);
    }
    Ok(())
}

fn render_bibliography_item_html(
    item: &BibliographyItem,
    bibliography: &RenderedBibliography,
    index: usize,
    output: &mut String,
) -> fmt::Result {
    output.push_str("<div class=\"csl-entry\" data-key=\"");
    write_html_escaped(output, &item.key);
    output.push_str("\" style=\"");
    if bibliography.second_field_align.is_some() {
        output.push_str("display:contents;");
    } else {
        if index > 0 {
            write!(output, "margin-top:{}em;", bibliography.entry_spacing)?;
        }
        if bibliography.hanging_indent {
            output.push_str("padding-left:2em;text-indent:-2em;");
        }
    }
    output.push_str("\">");
    if let Some(first_field) = &item.first_field {
        output.push_str("<div class=\"csl-left-margin\" style=\"grid-column:1;justify-self:end;white-space:nowrap;\">");
        render_child_html(first_field, output)?;
        output.push_str("</div><div class=\"csl-right-inline\" style=\"grid-column:2;\">");
        render_children_html(&item.content, output)?;
        output.push_str("</div>");
    } else {
        render_children_html(&item.content, output)?;
    }
    output.push_str("</div>");
    Ok(())
}
