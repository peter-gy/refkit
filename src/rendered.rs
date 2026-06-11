use std::fmt::{self, Write as _};
use std::sync::OnceLock;

use hayagriva::citationberg::{
    Display, FontStyle, FontVariant, FontWeight, TextDecoration, VerticalAlign,
};
use hayagriva::{BufWriteFormat, ElemChild, ElemChildren, RenderedCitation};
use pyo3::prelude::*;
use pyo3::types::PyAny;
use serde::Serialize;
use serde_json::json;

use crate::public_strings::{
    display_name, elem_meta_name, font_style_name, font_variant_name, font_weight_name,
    formatting_summary, quoted, text_decoration_name, vertical_align_name,
};
use crate::{RefkitError, json_to_py};

#[pyclass(module = "refkit", skip_from_py_object)]
pub struct Rendered {
    #[pyo3(get)]
    pub(crate) text: String,
    #[pyo3(get)]
    html: String,
    tree: RenderedTree,
    tree_json: OnceLock<String>,
}

pub(crate) enum RenderedTree {
    Empty,
    Citation(ElemChildren),
    Bibliography(Vec<hayagriva::BibliographyItem>),
}

#[pymethods]
impl Rendered {
    #[getter]
    fn tree(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let tree_json = py.detach(|| self.tree_json().to_string());
        json_to_py(py, &tree_json)
    }

    fn to_text(&self) -> String {
        self.text.clone()
    }

    fn to_html(&self) -> String {
        self.html.clone()
    }

    fn to_tree(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.tree(py)
    }

    fn __repr__(&self) -> String {
        format!("Rendered(text={})", quoted(&preview(&self.text)))
    }
}

impl Rendered {
    pub(crate) fn new(text: String, html: String, tree: RenderedTree) -> Self {
        Self {
            text,
            html,
            tree,
            tree_json: OnceLock::new(),
        }
    }

    fn tree_json(&self) -> &str {
        self.tree_json
            .get_or_init(|| rendered_tree_to_json(&self.tree))
            .as_str()
    }
}

pub(crate) fn rendered_from_citation(citation: &RenderedCitation) -> PyResult<Rendered> {
    let text = elem_children_to_string(&citation.citation, BufWriteFormat::Plain)?;
    let html = elem_children_to_html(&citation.citation)?;
    Ok(Rendered::new(
        text,
        html,
        RenderedTree::Citation(citation.citation.clone()),
    ))
}

pub(crate) fn rendered_from_bibliography(
    bibliography: Option<hayagriva::RenderedBibliography>,
) -> PyResult<Rendered> {
    let Some(bibliography) = bibliography else {
        return Ok(Rendered::new(
            String::new(),
            String::new(),
            RenderedTree::Empty,
        ));
    };

    let items = bibliography.items;
    let mut text = String::with_capacity(items.len() * 224);
    let mut html = String::with_capacity(items.len() * 384);
    for item in &items {
        if !text.is_empty() {
            text.push('\n');
        }
        write_bibliography_item_text(item, &mut text)?;

        render_bibliography_item_html(item, &mut html)
            .map_err(|err| RefkitError::new_err(err.to_string()))?;
    }

    Ok(Rendered::new(text, html, RenderedTree::Bibliography(items)))
}

pub(crate) fn elem_children_to_string(
    children: &ElemChildren,
    format: BufWriteFormat,
) -> PyResult<String> {
    let mut output = String::new();
    children
        .write_buf(&mut output, format)
        .map_err(|err| RefkitError::new_err(err.to_string()))?;
    Ok(output)
}

pub(crate) fn elem_children_to_html(children: &ElemChildren) -> PyResult<String> {
    let mut output = String::new();
    render_children_html(children, &mut output)
        .map_err(|err| RefkitError::new_err(err.to_string()))?;
    Ok(output)
}

fn write_bibliography_item_text(
    item: &hayagriva::BibliographyItem,
    output: &mut String,
) -> PyResult<()> {
    let item_start = output.len();
    if let Some(first_field) = &item.first_field {
        first_field
            .write_buf(output, BufWriteFormat::Plain)
            .map_err(|err| RefkitError::new_err(err.to_string()))?;
    }

    let content = elem_children_to_string(&item.content, BufWriteFormat::Plain)?;
    if output.len() > item_start && !content.is_empty() {
        output.push(' ');
    }
    output.push_str(&content);
    Ok(())
}

fn render_bibliography_item_html(
    item: &hayagriva::BibliographyItem,
    output: &mut String,
) -> fmt::Result {
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

fn safe_href(value: &str) -> Option<&str> {
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

fn rendered_tree_to_json(tree: &RenderedTree) -> String {
    match tree {
        RenderedTree::Empty => "[]".to_string(),
        RenderedTree::Citation(children) => serde_json::to_string(&children_to_tree(children))
            .expect("rendered citation tree is JSON serializable"),
        RenderedTree::Bibliography(items) => {
            let tree_items = items
                .iter()
                .map(bibliography_item_to_tree)
                .collect::<Vec<_>>();
            serde_json::to_string(&tree_items)
                .expect("rendered bibliography tree is JSON serializable")
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind")]
enum TreeNode {
    Text {
        text: String,
        formatting: TreeFormatting,
    },
    Element {
        display: Option<&'static str>,
        meta: Option<&'static str>,
        children: Vec<TreeNode>,
    },
    Markup {
        value: String,
    },
    Link {
        text: String,
        url: String,
        formatting: TreeFormatting,
    },
    Transparent {
        cite_idx: usize,
        format: String,
    },
}

#[derive(Debug, Serialize)]
struct TreeFormatting {
    font_style: &'static str,
    font_variant: &'static str,
    font_weight: &'static str,
    text_decoration: &'static str,
    vertical_align: &'static str,
}

fn children_to_tree(children: &ElemChildren) -> Vec<TreeNode> {
    children.0.iter().map(child_to_tree).collect()
}

fn bibliography_item_children_to_tree(item: &hayagriva::BibliographyItem) -> Vec<TreeNode> {
    let mut children = Vec::new();
    if let Some(first_field) = &item.first_field {
        children.push(child_to_tree(first_field));
    }
    children.extend(children_to_tree(&item.content));
    children
}

fn bibliography_item_to_tree(item: &hayagriva::BibliographyItem) -> serde_json::Value {
    let first_field = item.first_field.as_ref().map(child_to_tree);
    json!({
        "kind": "bibliography-entry",
        "key": &item.key,
        "first_field": first_field,
        "children": bibliography_item_children_to_tree(item),
    })
}

fn child_to_tree(child: &ElemChild) -> TreeNode {
    match child {
        ElemChild::Text(text) => TreeNode::Text {
            text: text.text.clone(),
            formatting: formatting_to_tree(text.formatting),
        },
        ElemChild::Elem(elem) => TreeNode::Element {
            display: elem.display.map(display_name),
            meta: elem.meta.as_ref().map(elem_meta_name),
            children: children_to_tree(&elem.children),
        },
        ElemChild::Markup(value) => TreeNode::Markup {
            value: value.clone(),
        },
        ElemChild::Link { text, url } => match safe_href(url) {
            Some(href) => TreeNode::Link {
                text: text.text.clone(),
                url: href.to_string(),
                formatting: formatting_to_tree(text.formatting),
            },
            None => TreeNode::Text {
                text: text.text.clone(),
                formatting: formatting_to_tree(text.formatting),
            },
        },
        ElemChild::Transparent { cite_idx, format } => TreeNode::Transparent {
            cite_idx: *cite_idx,
            format: formatting_summary(*format),
        },
    }
}

fn formatting_to_tree(formatting: hayagriva::Formatting) -> TreeFormatting {
    TreeFormatting {
        font_style: font_style_name(formatting.font_style),
        font_variant: font_variant_name(formatting.font_variant),
        font_weight: font_weight_name(formatting.font_weight),
        text_decoration: text_decoration_name(formatting.text_decoration),
        vertical_align: vertical_align_name(formatting.vertical_align),
    }
}

fn preview(value: &str) -> String {
    const LIMIT: usize = 60;
    if value.chars().count() <= LIMIT {
        return value.to_string();
    }
    let mut output: String = value.chars().take(LIMIT).collect();
    output.push_str("...");
    output
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
    use hayagriva::citationberg::Display;
    use hayagriva::{ElemChild, ElemChildren};
    use serde_json::Value;

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
    fn write_html_escaped_covers_special_characters() {
        let mut output = String::new();

        write_html_escaped(&mut output, "A&B <tag> \"quote\" 'apostrophe'");

        assert_eq!(
            output,
            "A&amp;B &lt;tag&gt; &quot;quote&quot; &#39;apostrophe&#39;"
        );
    }

    #[test]
    fn preview_preserves_short_text_and_truncates_long_text() {
        assert_eq!(preview("short"), "short");
        assert_eq!(
            preview(
                "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz"
            ),
            "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefgh..."
        );
    }

    #[test]
    fn rendered_empty_tree_serializes_to_empty_array() {
        assert_eq!(rendered_tree_to_json(&RenderedTree::Empty), "[]");
    }

    #[test]
    fn citation_tree_serializes_public_node_shapes() {
        let children = ElemChildren(vec![
            ElemChild::Text(formatted("plain")),
            ElemChild::Elem(hayagriva::Elem {
                children: ElemChildren(vec![ElemChild::Text(formatted("nested"))]),
                display: Some(Display::Block),
                meta: Some(hayagriva::ElemMeta::Text),
            }),
            ElemChild::Markup("<raw/>".to_string()),
            ElemChild::Link {
                text: formatted("safe"),
                url: "https://example.com".to_string(),
            },
            ElemChild::Link {
                text: formatted("unsafe"),
                url: "javascript:alert(1)".to_string(),
            },
            ElemChild::Transparent {
                cite_idx: 7,
                format: hayagriva::Formatting::default(),
            },
        ]);

        let tree: Value =
            serde_json::from_str(&rendered_tree_to_json(&RenderedTree::Citation(children)))
                .expect("citation tree should serialize");
        let nodes = tree.as_array().expect("citation tree is a JSON array");

        assert_eq!(nodes[0]["kind"], "Text");
        assert_eq!(nodes[0]["text"], "plain");
        assert_eq!(nodes[1]["kind"], "Element");
        assert_eq!(nodes[1]["display"], "Block");
        assert_eq!(nodes[1]["meta"], "Text");
        assert_eq!(nodes[1]["children"][0]["text"], "nested");
        assert_eq!(nodes[2]["kind"], "Markup");
        assert_eq!(nodes[2]["value"], "<raw/>");
        assert_eq!(nodes[3]["kind"], "Link");
        assert_eq!(nodes[3]["url"], "https://example.com");
        assert_eq!(nodes[4]["kind"], "Text");
        assert_eq!(nodes[4]["text"], "unsafe");
        assert_eq!(nodes[5]["kind"], "Transparent");
        assert_eq!(nodes[5]["cite_idx"], 7);
        assert_eq!(nodes[5]["format"], "Normal");
    }

    #[test]
    fn bibliography_tree_serializes_entries_and_first_field() {
        let items = vec![hayagriva::BibliographyItem {
            key: "doe2024".to_string(),
            first_field: Some(ElemChild::Text(formatted("[1]"))),
            content: ElemChildren(vec![ElemChild::Text(formatted("Doe, 2024."))]),
        }];

        let tree: Value =
            serde_json::from_str(&rendered_tree_to_json(&RenderedTree::Bibliography(items)))
                .expect("bibliography tree should serialize");
        let entries = tree.as_array().expect("bibliography tree is a JSON array");
        let entry = &entries[0];

        assert_eq!(entry["kind"], "bibliography-entry");
        assert_eq!(entry["key"], "doe2024");
        assert_eq!(entry["first_field"]["kind"], "Text");
        assert_eq!(entry["first_field"]["text"], "[1]");
        assert_eq!(entry["children"][0]["text"], "[1]");
        assert_eq!(entry["children"][1]["text"], "Doe, 2024.");
    }

    #[test]
    fn default_formatting_maps_to_public_tree_strings() {
        let formatting = formatting_to_tree(hayagriva::Formatting::default());

        assert_eq!(formatting.font_style, "Normal");
        assert_eq!(formatting.font_variant, "Normal");
        assert_eq!(formatting.font_weight, "Normal");
        assert_eq!(formatting.text_decoration, "None");
        assert_eq!(formatting.vertical_align, "None");
    }

    fn formatted(text: &str) -> hayagriva::Formatted {
        hayagriva::Formatted {
            text: text.to_string(),
            formatting: hayagriva::Formatting::default(),
        }
    }
}
