use std::fmt::{self, Write as _};

use hayagriva::citationberg::{
    Display, FontStyle, FontVariant, FontWeight, TextDecoration, VerticalAlign,
};
use hayagriva::{ElemChild, ElemChildren};

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

pub(crate) fn render_children_html(children: &ElemChildren, output: &mut String) -> fmt::Result {
    for child in &children.0 {
        render_child_html(child, output)?;
    }
    Ok(())
}

pub(crate) fn render_child_html(child: &ElemChild, output: &mut String) -> fmt::Result {
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
