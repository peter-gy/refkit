use std::sync::OnceLock;

use hayagriva::{BibliographyItem, BufWriteFormat, ElemChild, ElemChildren, RenderedCitation};

use crate::RenderedOutput;
use crate::render::{
    bibliography_to_text_html, elem_children_to_html, elem_children_to_string, safe_href,
};

#[derive(Debug, Clone)]
enum RenderedTree {
    Empty,
    Citation {
        children: ElemChildren,
        keys: Vec<String>,
    },
    Bibliography(Vec<BibliographyItem>),
}

#[derive(Debug, Clone)]
pub struct RenderedRecord {
    pub text: String,
    pub html: String,
    pub layout: Option<BibliographyLayout>,
    tree: RenderedTree,
    nodes: OnceLock<Vec<RenderedNode>>,
}

impl RenderedRecord {
    fn new(
        text: String,
        html: String,
        layout: Option<BibliographyLayout>,
        tree: RenderedTree,
    ) -> Self {
        Self {
            text,
            html,
            layout,
            tree,
            nodes: OnceLock::new(),
        }
    }

    pub fn output(&self) -> RenderedOutput {
        RenderedOutput {
            text: self.text.clone(),
            html: self.html.clone(),
        }
    }

    pub fn tree_nodes(&self) -> &[RenderedNode] {
        self.nodes.get_or_init(|| match &self.tree {
            RenderedTree::Empty => Vec::new(),
            RenderedTree::Citation { children, keys } => children_to_tree(children, keys),
            RenderedTree::Bibliography(items) => {
                items.iter().map(bibliography_item_to_tree).collect()
            }
        })
    }
}

pub(crate) fn rendered_record_from_citation(
    citation: &RenderedCitation,
    keys: Vec<String>,
) -> Result<RenderedRecord, String> {
    Ok(RenderedRecord::new(
        elem_children_to_string(&citation.citation, BufWriteFormat::Plain)?,
        elem_children_to_html(&citation.citation)?,
        None,
        RenderedTree::Citation {
            children: citation.citation.clone(),
            keys,
        },
    ))
}

pub(crate) fn rendered_record_from_bibliography(
    bibliography: Option<hayagriva::RenderedBibliography>,
) -> Result<RenderedRecord, String> {
    let Some(bibliography) = bibliography else {
        return Ok(RenderedRecord::new(
            String::new(),
            String::new(),
            None,
            RenderedTree::Empty,
        ));
    };
    let layout = BibliographyLayout {
        hanging_indent: bibliography.hanging_indent,
        second_field_align: bibliography
            .second_field_align
            .map(SecondFieldAlign::from_engine),
        line_spacing: bibliography.line_spacing.get(),
        entry_spacing: bibliography.entry_spacing,
    };
    let (text, html) = bibliography_to_text_html(&bibliography)?;
    Ok(RenderedRecord::new(
        text,
        html,
        Some(layout),
        RenderedTree::Bibliography(bibliography.items),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BibliographyLayout {
    pub hanging_indent: bool,
    pub second_field_align: Option<SecondFieldAlign>,
    pub line_spacing: i16,
    pub entry_spacing: i16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderedNode {
    Text {
        text: String,
        formatting: RenderedFormatting,
    },
    Element {
        display: Option<RenderedDisplay>,
        meta: Option<RenderedMeta>,
        children: Vec<RenderedNode>,
    },
    Markup {
        value: String,
    },
    Link {
        text: String,
        url: String,
        formatting: RenderedFormatting,
    },
    Transparent {
        cite_idx: usize,
        formatting: RenderedFormatting,
    },
    BibliographyEntry {
        key: String,
        label: Option<Box<RenderedNode>>,
        content: Vec<RenderedNode>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderedMeta {
    Names { roles: Vec<String> },
    Date,
    Text,
    Number,
    Label,
    CitationNumber,
    Name { role: String, index: usize },
    Entry { key: String, item_index: usize },
    CitationLabel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderedFormatting {
    pub font_style: FontStyle,
    pub font_variant: FontVariant,
    pub font_weight: FontWeight,
    pub text_decoration: TextDecoration,
    pub vertical_align: VerticalAlign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Italic,
}

impl FontStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Italic => "Italic",
        }
    }
}

impl FontStyle {
    fn from_engine(value: hayagriva::citationberg::FontStyle) -> Self {
        match value {
            hayagriva::citationberg::FontStyle::Normal => Self::Normal,
            hayagriva::citationberg::FontStyle::Italic => Self::Italic,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontVariant {
    Normal,
    SmallCaps,
}

impl FontVariant {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::SmallCaps => "SmallCaps",
        }
    }
}

impl FontVariant {
    fn from_engine(value: hayagriva::citationberg::FontVariant) -> Self {
        match value {
            hayagriva::citationberg::FontVariant::Normal => Self::Normal,
            hayagriva::citationberg::FontVariant::SmallCaps => Self::SmallCaps,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Normal,
    Bold,
    Light,
}

impl FontWeight {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Bold => "Bold",
            Self::Light => "Light",
        }
    }
}

impl FontWeight {
    fn from_engine(value: hayagriva::citationberg::FontWeight) -> Self {
        match value {
            hayagriva::citationberg::FontWeight::Normal => Self::Normal,
            hayagriva::citationberg::FontWeight::Bold => Self::Bold,
            hayagriva::citationberg::FontWeight::Light => Self::Light,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextDecoration {
    None,
    Underline,
}

impl TextDecoration {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Underline => "Underline",
        }
    }
}

impl TextDecoration {
    fn from_engine(value: hayagriva::citationberg::TextDecoration) -> Self {
        match value {
            hayagriva::citationberg::TextDecoration::None => Self::None,
            hayagriva::citationberg::TextDecoration::Underline => Self::Underline,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    None,
    Baseline,
    Sup,
    Sub,
}

impl VerticalAlign {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Baseline => "Baseline",
            Self::Sup => "Sup",
            Self::Sub => "Sub",
        }
    }
}

impl VerticalAlign {
    fn from_engine(value: hayagriva::citationberg::VerticalAlign) -> Self {
        match value {
            hayagriva::citationberg::VerticalAlign::None => Self::None,
            hayagriva::citationberg::VerticalAlign::Baseline => Self::Baseline,
            hayagriva::citationberg::VerticalAlign::Sup => Self::Sup,
            hayagriva::citationberg::VerticalAlign::Sub => Self::Sub,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderedDisplay {
    Block,
    LeftMargin,
    RightInline,
    Indent,
}

impl RenderedDisplay {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Block => "Block",
            Self::LeftMargin => "LeftMargin",
            Self::RightInline => "RightInline",
            Self::Indent => "Indent",
        }
    }
}

impl RenderedDisplay {
    fn from_engine(value: hayagriva::citationberg::Display) -> Self {
        match value {
            hayagriva::citationberg::Display::Block => Self::Block,
            hayagriva::citationberg::Display::LeftMargin => Self::LeftMargin,
            hayagriva::citationberg::Display::RightInline => Self::RightInline,
            hayagriva::citationberg::Display::Indent => Self::Indent,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondFieldAlign {
    Margin,
    Flush,
}

impl SecondFieldAlign {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Margin => "Margin",
            Self::Flush => "Flush",
        }
    }
}

impl SecondFieldAlign {
    fn from_engine(value: hayagriva::citationberg::SecondFieldAlign) -> Self {
        match value {
            hayagriva::citationberg::SecondFieldAlign::Margin => Self::Margin,
            hayagriva::citationberg::SecondFieldAlign::Flush => Self::Flush,
        }
    }
}

fn children_to_tree(children: &ElemChildren, keys: &[String]) -> Vec<RenderedNode> {
    children
        .0
        .iter()
        .map(|child| child_to_tree(child, keys))
        .collect()
}

fn bibliography_item_to_tree(item: &BibliographyItem) -> RenderedNode {
    RenderedNode::BibliographyEntry {
        key: item.key.clone(),
        label: item
            .first_field
            .as_ref()
            .map(|field| Box::new(child_to_tree(field, &[]))),
        content: children_to_tree(&item.content, &[]),
    }
}

fn child_to_tree(child: &ElemChild, keys: &[String]) -> RenderedNode {
    match child {
        ElemChild::Text(text) => RenderedNode::Text {
            text: text.text.clone(),
            formatting: RenderedFormatting::from_engine(text.formatting),
        },
        ElemChild::Elem(elem) => RenderedNode::Element {
            display: elem.display.map(RenderedDisplay::from_engine),
            meta: elem.meta.as_ref().and_then(|meta| metadata(meta, keys)),
            children: children_to_tree(&elem.children, keys),
        },
        ElemChild::Markup(value) => RenderedNode::Markup {
            value: value.clone(),
        },
        ElemChild::Link { text, url } => match safe_href(url) {
            Some(href) => RenderedNode::Link {
                text: text.text.clone(),
                url: href.to_string(),
                formatting: RenderedFormatting::from_engine(text.formatting),
            },
            None => RenderedNode::Text {
                text: text.text.clone(),
                formatting: RenderedFormatting::from_engine(text.formatting),
            },
        },
        ElemChild::Transparent { cite_idx, format } => RenderedNode::Transparent {
            cite_idx: *cite_idx,
            formatting: RenderedFormatting::from_engine(*format),
        },
    }
}

fn metadata(meta: &hayagriva::ElemMeta, keys: &[String]) -> Option<RenderedMeta> {
    Some(match meta {
        hayagriva::ElemMeta::Names(names) => RenderedMeta::Names {
            roles: names.iter().map(|(_, role)| role.to_string()).collect(),
        },
        hayagriva::ElemMeta::Date => RenderedMeta::Date,
        hayagriva::ElemMeta::Text => RenderedMeta::Text,
        hayagriva::ElemMeta::Number => RenderedMeta::Number,
        hayagriva::ElemMeta::Label => RenderedMeta::Label,
        hayagriva::ElemMeta::CitationNumber => RenderedMeta::CitationNumber,
        hayagriva::ElemMeta::Name(role, index) => RenderedMeta::Name {
            role: role.to_string(),
            index: *index,
        },
        hayagriva::ElemMeta::Entry(index) => RenderedMeta::Entry {
            key: keys.get(*index)?.clone(),
            item_index: *index,
        },
        hayagriva::ElemMeta::CitationLabel => RenderedMeta::CitationLabel,
    })
}

impl RenderedFormatting {
    fn from_engine(value: hayagriva::Formatting) -> Self {
        Self {
            font_style: FontStyle::from_engine(value.font_style),
            font_variant: FontVariant::from_engine(value.font_variant),
            font_weight: FontWeight::from_engine(value.font_weight),
            text_decoration: TextDecoration::from_engine(value.text_decoration),
            vertical_align: VerticalAlign::from_engine(value.vertical_align),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_preserves_opaque_fragments_and_filters_links() {
        let text = hayagriva::Formatted {
            text: "label".to_string(),
            formatting: hayagriva::Formatting::default(),
        };
        let children = ElemChildren(vec![
            ElemChild::Markup("<fragment>".to_string()),
            ElemChild::Link {
                text: text.clone(),
                url: "https://example.com".to_string(),
            },
            ElemChild::Link {
                text,
                url: "data:text/html,fragment".to_string(),
            },
            ElemChild::Transparent {
                cite_idx: 2,
                format: hayagriva::Formatting::default(),
            },
        ]);
        let nodes = children_to_tree(&children, &[]);
        assert!(matches!(&nodes[0], RenderedNode::Markup { value } if value == "<fragment>"));
        assert!(
            matches!(&nodes[1], RenderedNode::Link { url, .. } if url == "https://example.com")
        );
        assert!(matches!(&nodes[2], RenderedNode::Text { text, .. } if text == "label"));
        assert!(
            matches!(&nodes[3], RenderedNode::Transparent { cite_idx: 2, formatting } if formatting.font_style == FontStyle::Normal)
        );
        assert_eq!(
            elem_children_to_html(&children).unwrap(),
            "&lt;fragment&gt;<a href=\"https://example.com\">label</a>label"
        );
    }
}
