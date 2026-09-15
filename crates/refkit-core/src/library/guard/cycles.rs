use std::collections::HashMap;
use std::ops::Range;

use biblatex::{Field, RawBibliography, RawChunk};

use super::{Diagnostic, MAX_DEPTH, MAX_STEPS, limit};

pub(super) type Graph = Vec<Vec<(usize, Range<usize>, String)>>;

struct BackEdge {
    root: usize,
    source: usize,
    edge: usize,
}

#[expect(
    clippy::indexing_slicing,
    reason = "Both graph builders resolve every target through the same node enumeration and supply roots from that enumeration. Pending vertices and edge positions come only from those validated graph entries."
)]
fn back_edges(graph: &Graph, roots: &[usize]) -> Result<Vec<BackEdge>, BackEdge> {
    let mut state = vec![0u8; graph.len()];
    let mut cycles = Vec::new();
    let mut steps = 0;
    let mut pending = Vec::new();
    for (root, &start) in roots.iter().enumerate() {
        if state[start] != 0 {
            continue;
        }
        state[start] = 1;
        pending.push((start, 0));
        while let Some((source, next)) = pending.last_mut() {
            let Some(&(target, _, _)) = graph[*source].get(*next) else {
                state[*source] = 2;
                pending.pop();
                continue;
            };
            let edge = BackEdge {
                root,
                source: *source,
                edge: *next,
            };
            *next += 1;
            steps += 1;
            if steps > MAX_STEPS || (state[target] == 0 && pending.len() >= MAX_DEPTH) {
                return Err(edge);
            }
            match state[target] {
                1 => cycles.push(edge),
                0 => {
                    state[target] = 1;
                    pending.push((target, 0));
                }
                _ => {}
            }
        }
    }
    Ok(cycles)
}

fn macro_edges(
    field: &Field<'_>,
    definitions: &HashMap<&str, usize>,
) -> Vec<(usize, Range<usize>, String)> {
    field
        .iter()
        .filter_map(|chunk| {
            let RawChunk::Abbreviation(name) = chunk.v else {
                return None;
            };
            definitions
                .get(name)
                .map(|&target| (target, chunk.span.clone(), name.to_string()))
        })
        .collect()
}

#[expect(
    clippy::indexing_slicing,
    reason = "The scanner returns only visited graph edge positions and indices into the supplied root list. Each root stores the entry and field that initiated its traversal."
)]
pub(super) fn abbreviations(raw: &RawBibliography<'_>) -> Result<Vec<Diagnostic>, Diagnostic> {
    if raw.abbreviations.is_empty() {
        return Ok(Vec::new());
    }
    let definitions = raw
        .abbreviations
        .iter()
        .enumerate()
        .map(|(index, pair)| (pair.key.v, index))
        .collect();
    let mut graph = raw
        .abbreviations
        .iter()
        .map(|pair| macro_edges(&pair.value.v, &definitions))
        .collect::<Graph>();
    let mut roots = Vec::new();
    for entry in &raw.entries {
        for field in &entry.v.fields {
            let edges = macro_edges(&field.value.v, &definitions);
            if !edges.is_empty() {
                roots.push((graph.len(), entry.v.key.v, field.key.v));
                graph.push(edges);
            }
        }
    }
    let root_indices = roots.iter().map(|root| root.0).collect::<Vec<_>>();
    let diagnostic = |edge: BackEdge, exhausted: bool| {
        let (_, span, name) = &graph[edge.source][edge.edge];
        let (_, entry, field) = roots[edge.root];
        let mut diagnostic = if exhausted {
            limit(
                Some(span.clone()),
                "bibliography expansion exceeds its dependency budget",
            )
        } else {
            Diagnostic::error(
                "cyclic_abbreviation",
                Some(span.clone()),
                format!("cyclic BibTeX abbreviation {name:?}"),
            )
        };
        diagnostic.entry = Some(entry.to_string());
        diagnostic.field = Some(field.to_ascii_lowercase());
        diagnostic
    };
    back_edges(&graph, &root_indices)
        .map(|edges| {
            edges
                .into_iter()
                .map(|edge| diagnostic(edge, false))
                .collect()
        })
        .map_err(|edge| diagnostic(edge, true))
}

#[expect(
    clippy::indexing_slicing,
    reason = "Reference graph nodes use the raw entry enumeration, with all edge targets resolved through that enumeration. The scanner returns only visited edge positions."
)]
pub(super) fn references(
    graph: &Graph,
    raw: &RawBibliography<'_>,
) -> Result<Vec<Diagnostic>, Diagnostic> {
    let roots = (0..graph.len()).collect::<Vec<_>>();
    let diagnostic = |edge: BackEdge, exhausted: bool| {
        let (target, span, field) = &graph[edge.source][edge.edge];
        let mut diagnostic = if exhausted {
            limit(
                Some(span.clone()),
                "bibliography inheritance exceeds its dependency budget",
            )
        } else {
            Diagnostic::error(
                "cyclic_reference",
                Some(span.clone()),
                format!(
                    "cyclic BibTeX reference to {:?}",
                    raw.entries[*target].v.key.v
                ),
            )
        };
        diagnostic.entry = Some(raw.entries[edge.source].v.key.v.to_string());
        diagnostic.field = Some(field.to_ascii_lowercase());
        diagnostic
    };
    back_edges(graph, &roots)
        .map(|edges| {
            edges
                .into_iter()
                .map(|edge| diagnostic(edge, false))
                .collect()
        })
        .map_err(|edge| diagnostic(edge, true))
}
