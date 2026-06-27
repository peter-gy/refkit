use hayagriva::{BibliographyItem, BufWriteFormat, ElemChild, ElemChildren, RenderedCitation};

use crate::render::{
    bibliography_to_text_html, elem_children_to_html, elem_children_to_string, safe_href,
};
use crate::{
    RenderedOutput, display_name, elem_meta_name, font_style_name, font_variant_name,
    font_weight_name, formatting_summary, text_decoration_name, vertical_align_name,
};

#[derive(Debug, Clone)]
enum RenderedTree {
    Empty,
    Citation(ElemChildren),
    Bibliography(Vec<BibliographyItem>),
}

#[derive(Debug, Clone)]
pub struct RenderedRecord {
    pub text: String,
    pub html: String,
    tree: RenderedTree,
}

impl RenderedRecord {
    fn new(text: String, html: String, tree: RenderedTree) -> Self {
        Self { text, html, tree }
    }

    pub fn output(&self) -> RenderedOutput {
        RenderedOutput {
            text: self.text.clone(),
            html: self.html.clone(),
        }
    }

    pub fn tree_nodes(&self) -> Vec<RenderedNode> {
        rendered_tree_nodes(&self.tree)
    }
}

pub fn rendered_record_from_citation(
    citation: &RenderedCitation,
) -> Result<RenderedRecord, String> {
    Ok(RenderedRecord::new(
        elem_children_to_string(&citation.citation, BufWriteFormat::Plain)?,
        elem_children_to_html(&citation.citation)?,
        RenderedTree::Citation(citation.citation.clone()),
    ))
}

pub fn rendered_record_from_bibliography(
    bibliography: Option<hayagriva::RenderedBibliography>,
) -> Result<RenderedRecord, String> {
    let Some(bibliography) = bibliography else {
        return Ok(RenderedRecord::new(
            String::new(),
            String::new(),
            RenderedTree::Empty,
        ));
    };

    let items = bibliography.items;
    let (text, html) = bibliography_to_text_html(&items)?;
    Ok(RenderedRecord::new(
        text,
        html,
        RenderedTree::Bibliography(items),
    ))
}

fn rendered_tree_nodes(tree: &RenderedTree) -> Vec<RenderedNode> {
    match tree {
        RenderedTree::Empty => Vec::new(),
        RenderedTree::Citation(children) => children_to_tree(children),
        RenderedTree::Bibliography(items) => items.iter().map(bibliography_item_to_tree).collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderedNode {
    Text {
        text: String,
        formatting: RenderedFormatting,
    },
    Element {
        display: Option<&'static str>,
        meta: Option<&'static str>,
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
        format: String,
    },
    BibliographyEntry {
        key: String,
        first_field: Option<Box<RenderedNode>>,
        children: Vec<RenderedNode>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderedFormatting {
    pub font_style: &'static str,
    pub font_variant: &'static str,
    pub font_weight: &'static str,
    pub text_decoration: &'static str,
    pub vertical_align: &'static str,
}

fn children_to_tree(children: &ElemChildren) -> Vec<RenderedNode> {
    children.0.iter().map(child_to_tree).collect()
}

fn bibliography_item_children_to_tree(item: &BibliographyItem) -> Vec<RenderedNode> {
    let mut children = Vec::new();
    if let Some(first_field) = &item.first_field {
        children.push(child_to_tree(first_field));
    }
    children.extend(children_to_tree(&item.content));
    children
}

fn bibliography_item_to_tree(item: &BibliographyItem) -> RenderedNode {
    RenderedNode::BibliographyEntry {
        key: item.key.clone(),
        first_field: item.first_field.as_ref().map(child_to_tree).map(Box::new),
        children: bibliography_item_children_to_tree(item),
    }
}

fn child_to_tree(child: &ElemChild) -> RenderedNode {
    match child {
        ElemChild::Text(text) => RenderedNode::Text {
            text: text.text.clone(),
            formatting: formatting_to_tree(text.formatting),
        },
        ElemChild::Elem(elem) => RenderedNode::Element {
            display: elem.display.map(display_name),
            meta: elem.meta.as_ref().map(elem_meta_name),
            children: children_to_tree(&elem.children),
        },
        ElemChild::Markup(value) => RenderedNode::Markup {
            value: value.clone(),
        },
        ElemChild::Link { text, url } => match safe_href(url) {
            Some(href) => RenderedNode::Link {
                text: text.text.clone(),
                url: href.to_string(),
                formatting: formatting_to_tree(text.formatting),
            },
            None => RenderedNode::Text {
                text: text.text.clone(),
                formatting: formatting_to_tree(text.formatting),
            },
        },
        ElemChild::Transparent { cite_idx, format } => RenderedNode::Transparent {
            cite_idx: *cite_idx,
            format: formatting_summary(*format),
        },
    }
}

fn formatting_to_tree(formatting: hayagriva::Formatting) -> RenderedFormatting {
    RenderedFormatting {
        font_style: font_style_name(formatting.font_style),
        font_variant: font_variant_name(formatting.font_variant),
        font_weight: font_weight_name(formatting.font_weight),
        text_decoration: text_decoration_name(formatting.text_decoration),
        vertical_align: vertical_align_name(formatting.vertical_align),
    }
}

#[cfg(test)]
mod tests {
    use hayagriva::citationberg::Display;
    use hayagriva::{ElemChild, ElemChildren};

    use super::*;

    #[test]
    fn rendered_empty_tree_has_no_nodes() {
        assert_eq!(rendered_tree_nodes(&RenderedTree::Empty), Vec::new());
    }

    #[test]
    fn citation_tree_contains_public_node_shapes() {
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

        let nodes = rendered_tree_nodes(&RenderedTree::Citation(children));

        assert!(matches!(&nodes[0], RenderedNode::Text { text, .. } if text == "plain"));
        assert!(
            matches!(&nodes[1], RenderedNode::Element { display, meta, children } if *display == Some("Block") && *meta == Some("Text") && matches!(&children[0], RenderedNode::Text { text, .. } if text == "nested"))
        );
        assert!(matches!(&nodes[2], RenderedNode::Markup { value } if value == "<raw/>"));
        assert!(
            matches!(&nodes[3], RenderedNode::Link { url, .. } if url == "https://example.com")
        );
        assert!(matches!(&nodes[4], RenderedNode::Text { text, .. } if text == "unsafe"));
        assert!(
            matches!(&nodes[5], RenderedNode::Transparent { cite_idx, format } if *cite_idx == 7 && format == "Normal")
        );
    }

    #[test]
    fn bibliography_tree_contains_entries_and_first_field() {
        let items = vec![BibliographyItem {
            key: "doe2024".to_string(),
            first_field: Some(ElemChild::Text(formatted("[1]"))),
            content: ElemChildren(vec![ElemChild::Text(formatted("Doe, 2024."))]),
        }];

        let tree = rendered_tree_nodes(&RenderedTree::Bibliography(items));
        let RenderedNode::BibliographyEntry {
            key,
            first_field,
            children,
        } = &tree[0]
        else {
            panic!("bibliography tree should contain entry nodes");
        };

        assert_eq!(key, "doe2024");
        assert!(
            matches!(first_field.as_deref(), Some(RenderedNode::Text { text, .. }) if text == "[1]")
        );
        assert!(matches!(&children[0], RenderedNode::Text { text, .. } if text == "[1]"));
        assert!(matches!(&children[1], RenderedNode::Text { text, .. } if text == "Doe, 2024."));
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
