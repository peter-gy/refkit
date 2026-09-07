use std::collections::HashMap;

use hayagriva::citationberg::{
    IndependentStyle, LayoutRenderingElement, Sort, SortKey, TextTarget,
};
use quick_xml::{Reader, events::Event};

use super::StyleError;
use crate::quoted;

const MAX_STYLE_BYTES: usize = 2 * 1024 * 1024;
const MAX_STYLE_NODES: usize = 100_000;
const MAX_STYLE_DEPTH: usize = 64;
const MAX_STYLE_ATTRIBUTES: usize = 256;

pub(super) fn validate_xml_budget(xml: &str) -> Result<(), StyleError> {
    if xml.len() > MAX_STYLE_BYTES {
        return Err(StyleError::InvalidXml(
            "source exceeds 2097152 bytes".to_string(),
        ));
    }
    let mut reader = Reader::from_str(xml);
    let mut depth = 0_usize;
    let mut nodes = 0_usize;
    loop {
        let event = reader
            .read_event()
            .map_err(|error| StyleError::InvalidXml(error.to_string()))?;
        if let Event::Start(element) | Event::Empty(element) = &event {
            for (index, attribute) in element.attributes().with_checks(false).enumerate() {
                if index >= MAX_STYLE_ATTRIBUTES {
                    return Err(StyleError::InvalidXml(
                        "element exceeds 256 attributes".to_string(),
                    ));
                }
                attribute.map_err(|error| StyleError::InvalidXml(error.to_string()))?;
            }
        }
        match event {
            Event::Eof => break,
            Event::End(_) => {
                depth = depth.saturating_sub(1);
                continue;
            }
            Event::Start(_) => {
                depth += 1;
            }
            Event::Empty(_) if depth + 1 > MAX_STYLE_DEPTH => {
                return Err(StyleError::InvalidXml(
                    "nesting exceeds 64 elements".to_string(),
                ));
            }
            _ => {}
        }
        nodes += 1;
        if depth > MAX_STYLE_DEPTH {
            return Err(StyleError::InvalidXml(
                "nesting exceeds 64 elements".to_string(),
            ));
        }
        if nodes > MAX_STYLE_NODES {
            return Err(StyleError::InvalidXml(
                "source exceeds 100000 XML nodes".to_string(),
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_macros(style: &IndependentStyle) -> Result<(), StyleError> {
    let mut indices = HashMap::new();
    for (index, definition) in style.macros.iter().enumerate() {
        if indices.insert(definition.name.as_str(), index).is_some() {
            return Err(StyleError::InvalidMacro(format!(
                "duplicate definition {}",
                quoted(&definition.name)
            )));
        }
    }
    let mut graphs = Vec::with_capacity(style.macros.len() + 1);
    for definition in &style.macros {
        graphs.push(macro_usage(&definition.children));
    }
    let mut root = macro_usage(&style.citation.layout.elements);
    add_sort_macros(style.citation.sort.as_ref(), &mut root.references);
    if let Some(bibliography) = &style.bibliography {
        let bibliography_usage = macro_usage(&bibliography.layout.elements);
        root.references.extend(bibliography_usage.references);
        root.depth = root.depth.max(bibliography_usage.depth);
        add_sort_macros(bibliography.sort.as_ref(), &mut root.references);
    }
    graphs.push(root);
    let edges = graphs
        .iter()
        .map(|usage| {
            usage
                .references
                .iter()
                .map(|&(name, nesting)| {
                    indices
                        .get(name)
                        .copied()
                        .map(|index| (index, nesting))
                        .ok_or_else(|| {
                            StyleError::InvalidMacro(format!("missing definition {}", quoted(name)))
                        })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut state = vec![0_u8; graphs.len()];
    let mut depth = vec![0_usize; graphs.len()];
    let mut cost = vec![0_usize; graphs.len()];
    for start in 0..graphs.len() {
        if state[start] == 2 {
            continue;
        }
        let mut stack = vec![(start, false)];
        let mut path: Vec<usize> = Vec::new();
        while let Some((index, finishing)) = stack.pop() {
            if finishing {
                let expanded = if let Some(definition) = style.macros.get(index) {
                    expanded_elements(&definition.children, &indices, &cost)
                } else {
                    let citation =
                        expanded_elements(&style.citation.layout.elements, &indices, &cost)
                            .saturating_add(expanded_sort(
                                style.citation.sort.as_ref(),
                                &indices,
                                &cost,
                            ));
                    style
                        .bibliography
                        .as_ref()
                        .map_or(citation, |bibliography| {
                            citation
                                .saturating_add(expanded_elements(
                                    &bibliography.layout.elements,
                                    &indices,
                                    &cost,
                                ))
                                .saturating_add(expanded_sort(
                                    bibliography.sort.as_ref(),
                                    &indices,
                                    &cost,
                                ))
                        })
                };
                let mut longest = graphs[index].depth;
                for &(child, nesting) in &edges[index] {
                    longest = longest.max(depth[child] + nesting);
                }
                if longest > MAX_STYLE_DEPTH || expanded > MAX_STYLE_NODES {
                    return Err(StyleError::InvalidMacro(format!(
                        "expansion exceeds 64 nested elements or 100000 elements (depth {longest}, elements {expanded})"
                    )));
                }
                cost[index] = expanded;
                depth[index] = longest;
                state[index] = 2;
                path.pop();
            } else {
                match state[index] {
                    2 => continue,
                    1 => {
                        let mut cycle = path
                            .iter()
                            .skip_while(|&&node| node != index)
                            .map(|&node| quoted(&style.macros[node].name))
                            .collect::<Vec<_>>();
                        cycle.push(quoted(&style.macros[index].name));
                        return Err(StyleError::InvalidMacro(format!(
                            "cycle: {}",
                            cycle.join(" -> ")
                        )));
                    }
                    _ => {}
                }
                state[index] = 1;
                path.push(index);
                stack.push((index, true));
                for &(child, _) in edges[index].iter().rev() {
                    stack.push((child, false));
                }
            }
        }
    }
    Ok(())
}

fn expanded_elements(
    elements: &[LayoutRenderingElement],
    indices: &HashMap<&str, usize>,
    costs: &[usize],
) -> usize {
    elements
        .iter()
        .map(|element| {
            let children = match element {
                LayoutRenderingElement::Text(text) => match &text.target {
                    TextTarget::Macro { name } => costs[indices[name.as_str()]],
                    _ => 0,
                },
                LayoutRenderingElement::Group(group) => {
                    expanded_elements(&group.children, indices, costs)
                }
                LayoutRenderingElement::Names(names) => {
                    names.substitute().map_or(0, |substitute| {
                        expanded_elements(&substitute.children, indices, costs)
                    })
                }
                // A choose renders one branch, so alternatives share the expansion budget.
                LayoutRenderingElement::Choose(choose) => choose
                    .branches()
                    .map(|branch| expanded_elements(&branch.children, indices, costs))
                    .chain(
                        choose
                            .otherwise
                            .iter()
                            .map(|branch| expanded_elements(&branch.children, indices, costs)),
                    )
                    .max()
                    .unwrap_or(0),
                LayoutRenderingElement::Date(_)
                | LayoutRenderingElement::Number(_)
                | LayoutRenderingElement::Label(_) => 0,
            };
            children.saturating_add(1)
        })
        .fold(0, usize::saturating_add)
}

fn expanded_sort(sort: Option<&Sort>, indices: &HashMap<&str, usize>, costs: &[usize]) -> usize {
    sort.map_or(0, |sort| {
        sort.keys
            .iter()
            .map(|key| match key {
                SortKey::MacroName { name, .. } => costs[indices[name.as_str()]],
                SortKey::Variable { .. } => 1,
            })
            .fold(0, usize::saturating_add)
    })
}

struct MacroUsage<'a> {
    references: Vec<(&'a str, usize)>,
    depth: usize,
}

fn macro_usage<'a>(elements: &'a [LayoutRenderingElement]) -> MacroUsage<'a> {
    let mut pending = elements
        .iter()
        .map(|element| (element, 1))
        .collect::<Vec<_>>();
    let mut usage = MacroUsage {
        references: Vec::new(),
        depth: 0,
    };
    while let Some((element, depth)) = pending.pop() {
        usage.depth = usage.depth.max(depth);
        let mut append = |children: &'a [LayoutRenderingElement], increment| {
            pending.extend(children.iter().map(|child| (child, depth + increment)));
        };
        match element {
            LayoutRenderingElement::Text(text) => {
                if let TextTarget::Macro { name } = &text.target {
                    usage.references.push((name.as_str(), depth));
                }
            }
            LayoutRenderingElement::Group(group) => append(&group.children, 1),
            LayoutRenderingElement::Names(names) => {
                if let Some(substitute) = names.substitute() {
                    append(&substitute.children, 2);
                }
            }
            LayoutRenderingElement::Choose(choose) => {
                for branch in choose.branches() {
                    append(&branch.children, 2);
                }
                if let Some(branch) = &choose.otherwise {
                    append(&branch.children, 2);
                }
            }
            LayoutRenderingElement::Date(_)
            | LayoutRenderingElement::Number(_)
            | LayoutRenderingElement::Label(_) => {}
        }
    }
    usage
}

fn add_sort_macros<'a>(sort: Option<&'a Sort>, references: &mut Vec<(&'a str, usize)>) {
    if let Some(sort) = sort {
        for key in &sort.keys {
            if let SortKey::MacroName { name, .. } = key {
                references.push((name, 1));
            }
        }
    }
}
