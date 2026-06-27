use std::collections::HashMap;
use std::fmt::{self, Write as _};
use std::sync::OnceLock;

use hayagriva::citationberg::{
    Display, FontStyle, FontVariant, FontWeight, IndependentStyle, Locale as CslLocale, LocaleCode,
    TextDecoration, VerticalAlign,
};
use hayagriva::{
    BibliographyDriver, BibliographyItem, BibliographyRequest, BufWriteFormat, CitationItem,
    CitationRequest, ElemChild, ElemChildren, Library as HayLibrary, archive, standalone_citation,
};

use crate::library::Library;
use crate::quoted;
use crate::style::PreparedStyle;
use crate::style_analysis::can_fast_render_single_citations;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedOutput {
    pub text: String,
    pub html: String,
}

pub fn bundled_locales() -> &'static [CslLocale] {
    static LOCALES: OnceLock<Vec<CslLocale>> = OnceLock::new();
    LOCALES.get_or_init(archive::locales).as_slice()
}

pub fn render_library_citation(
    library: &Library,
    key: &str,
    style: &PreparedStyle,
    locale: Option<&str>,
) -> Result<RenderedOutput, String> {
    render_citation(library.inner(), key, style.inner.as_ref(), locale)
}

pub fn render_library_citation_each(
    library: &Library,
    keys: &[&str],
    style: &PreparedStyle,
    locale: Option<&str>,
) -> Result<Vec<RenderedOutput>, String> {
    render_citation_each(library.inner(), keys, style.inner.as_ref(), locale)
}

pub fn render_library_citation_group(
    library: &Library,
    keys: &[&str],
    style: &PreparedStyle,
    locale: Option<&str>,
) -> Result<RenderedOutput, String> {
    render_citation_group(library.inner(), keys, style.inner.as_ref(), locale)
}

pub fn render_library_bibliography(
    library: &Library,
    style: &PreparedStyle,
    locale: Option<&str>,
    all: bool,
) -> Result<RenderedOutput, String> {
    render_bibliography(library.inner(), style.inner.as_ref(), locale, all)
}

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
    if can_fast_render_single_citations(style) {
        if let Some(rendered) =
            render_independent_citation_each(library, keys, style, locale.clone(), locales)?
        {
            return Ok(rendered);
        }
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
        if let Some(existing_key) = key_by_text.get(&output.text) {
            if existing_key != key {
                return Ok(None);
            }
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

pub(crate) fn elem_children_to_string(
    children: &ElemChildren,
    format: BufWriteFormat,
) -> Result<String, String> {
    let mut output = String::new();
    children
        .write_buf(&mut output, format)
        .map_err(|err| err.to_string())?;
    Ok(output)
}

pub(crate) fn elem_children_to_html(children: &ElemChildren) -> Result<String, String> {
    let mut output = String::new();
    render_children_html(children, &mut output).map_err(|err| err.to_string())?;
    Ok(output)
}

pub(crate) fn safe_href(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let scheme_end = trimmed.find(':')?;
    let first_path_marker = trimmed.find(['/', '?', '#']).unwrap_or(usize::MAX);
    if scheme_end > first_path_marker {
        return None;
    }

    let scheme = trimmed[..scheme_end].to_ascii_lowercase();
    match scheme.as_str() {
        "http" | "https" | "mailto" => Some(trimmed),
        _ => None,
    }
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

fn render_children_html(children: &ElemChildren, output: &mut String) -> fmt::Result {
    for child in &children.0 {
        render_child_html(child, output)?;
    }
    Ok(())
}

fn render_child_html(child: &ElemChild, output: &mut String) -> fmt::Result {
    match child {
        ElemChild::Text(text) => render_formatted_html(text, output),
        ElemChild::Elem(elem) => render_elem_html(elem, output),
        ElemChild::Markup(value) => {
            write_html_escaped(output, value);
            Ok(())
        }
        ElemChild::Link { text, url } => {
            if let Some(href) = safe_href(url) {
                output.push_str("<a href=\"");
                write_html_escaped(output, href);
                output.push_str("\">");
                render_formatted_html(text, output)?;
                output.push_str("</a>");
            } else {
                render_formatted_html(text, output)?;
            }
            Ok(())
        }
        ElemChild::Transparent { .. } => Ok(()),
    }
}

fn render_elem_html(elem: &hayagriva::Elem, output: &mut String) -> fmt::Result {
    if let Some(display) = elem.display {
        let class_name = match display {
            Display::Block => "csl-block",
            Display::LeftMargin => "csl-left-margin",
            Display::RightInline => "csl-right-inline",
            Display::Indent => "csl-indent",
        };
        write!(output, "<div class=\"{class_name}\">")?;
    }

    render_children_html(&elem.children, output)?;

    if elem.display.is_some() {
        output.push_str("</div>");
    }
    Ok(())
}

fn render_formatted_html(text: &hayagriva::Formatted, output: &mut String) -> fmt::Result {
    let formatting = text.formatting;
    if formatting == hayagriva::Formatting::default() {
        write_html_escaped(output, &text.text);
        return Ok(());
    }

    let mut css = String::new();
    let mut suffix = String::new();

    match formatting.vertical_align {
        VerticalAlign::Sub => push_html_wrapper(output, &mut suffix, "<sub>", "</sub>"),
        VerticalAlign::Sup => push_html_wrapper(output, &mut suffix, "<sup>", "</sup>"),
        VerticalAlign::Baseline => {
            css.push_str("vertical-align: baseline;");
        }
        VerticalAlign::None => {}
    }

    match formatting.font_weight {
        FontWeight::Bold => {
            if text.text.chars().any(|c| !c.is_whitespace()) {
                push_html_wrapper(output, &mut suffix, "<b>", "</b>");
            }
        }
        FontWeight::Light => css.push_str("font-weight: lighter;"),
        FontWeight::Normal => {}
    }

    if formatting.font_style == FontStyle::Italic {
        push_html_wrapper(output, &mut suffix, "<i>", "</i>");
    }

    if formatting.font_variant == FontVariant::SmallCaps {
        css.push_str("font-variant: small-caps;");
    }

    if formatting.text_decoration == TextDecoration::Underline {
        push_html_wrapper(output, &mut suffix, "<u>", "</u>");
    }

    if !css.is_empty() {
        push_html_wrapper(
            output,
            &mut suffix,
            &format!("<span style=\"{css}\">"),
            "</span>",
        );
    }

    write_html_escaped(output, &text.text);
    output.push_str(&suffix);
    Ok(())
}

fn push_html_wrapper(output: &mut String, suffix: &mut String, start: &str, end: &str) {
    output.push_str(start);
    suffix.insert_str(0, end);
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

#[cfg(test)]
mod tests {
    use hayagriva::{ElemChild, ElemChildren};

    use crate::load_prepared_style;

    use super::*;

    #[test]
    fn safe_href_allows_only_link_schemes() {
        assert_eq!(
            safe_href(" https://example.com/path "),
            Some("https://example.com/path")
        );
        assert_eq!(safe_href("http://example.com"), Some("http://example.com"));
        assert_eq!(
            safe_href("mailto:dev@example.com"),
            Some("mailto:dev@example.com")
        );
        assert_eq!(safe_href("javascript:alert(1)"), None);
        assert_eq!(safe_href("data:text/html,hello"), None);
        assert_eq!(safe_href("/relative:with-colon"), None);
        assert_eq!(safe_href(""), None);
    }

    #[test]
    fn html_renderer_escapes_special_characters() {
        let children = ElemChildren(vec![ElemChild::Text(formatted(
            "A&B <tag> \"quote\" 'apostrophe'",
        ))]);

        assert_eq!(
            elem_children_to_html(&children).unwrap(),
            "A&amp;B &lt;tag&gt; &quot;quote&quot; &#39;apostrophe&#39;"
        );
    }

    #[test]
    fn html_renderer_suppresses_unsafe_links() {
        let children = ElemChildren(vec![ElemChild::Link {
            text: formatted("link"),
            url: "javascript:alert(1)".to_string(),
        }]);

        assert_eq!(elem_children_to_html(&children).unwrap(), "link");
    }

    #[test]
    fn renders_citation_text_and_html() {
        let library = Library::parse_source(
            "@article{doe2024, author = {Doe, Jane}, title = {Core}, year = {2024}}",
            "bibtex",
            false,
            false,
        )
        .unwrap();
        let style = load_prepared_style("apa").unwrap();

        let rendered =
            render_library_citation(&library, "doe2024", style.as_ref(), Some("en-US")).unwrap();

        assert!(rendered.text.contains("Doe"));
        assert!(rendered.html.contains("Doe"));
    }

    #[test]
    fn renders_citation_each_in_key_order() {
        let library = Library::parse_source(
            "@article{doe2024, author = {Doe, Jane}, title = {Core}, year = {2024}}
             @article{roe2023, author = {Roe, Richard}, title = {Edges}, year = {2023}}",
            "bibtex",
            false,
            false,
        )
        .unwrap();
        let style = load_prepared_style("apa").unwrap();

        let rendered = render_library_citation_each(
            &library,
            &["doe2024", "roe2023"],
            style.as_ref(),
            Some("en-US"),
        )
        .unwrap();

        assert_eq!(rendered.len(), 2);
        assert!(rendered[0].text.contains("Doe"));
        assert!(rendered[1].text.contains("Roe"));
        assert!(rendered[0].html.contains("Doe"));
        assert!(rendered[1].html.contains("Roe"));
    }

    #[test]
    fn citation_each_falls_back_for_ambiguous_fast_texts() {
        let library = Library::parse_source(
            "@article{doe2024a, author = {Doe, Jane}, title = {Alpha}, year = {2024}}
             @article{doe2024b, author = {Doe, Jane}, title = {Beta}, year = {2024}}",
            "bibtex",
            false,
            false,
        )
        .unwrap();
        let style = load_prepared_style("apa").unwrap();

        let rendered = render_library_citation_each(
            &library,
            &["doe2024a", "doe2024b"],
            style.as_ref(),
            Some("en-US"),
        )
        .unwrap();

        assert_eq!(rendered.len(), 2);
        assert_ne!(rendered[0].text, rendered[1].text);
    }

    fn formatted(text: &str) -> hayagriva::Formatted {
        hayagriva::Formatted {
            text: text.to_string(),
            formatting: hayagriva::Formatting::default(),
        }
    }
}
