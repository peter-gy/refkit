use biblatex::{RawChunk, Spanned};

pub(crate) fn decode_expression(
    expression: &str,
    abbreviations: &[biblatex::Pair<'_>],
    is_list: bool,
) -> Result<Vec<String>, String> {
    let document =
        crate::RawDocument::parse(&format!("@misc{{reference,value={expression}}}")).into_syntax();
    let [entry] = document.entries.as_slice() else {
        return Err("Expected one reference value expression".into());
    };
    let [field] = entry.fields.as_slice() else {
        return Err("Expected one reference value expression".into());
    };
    crate::library::normalize_reference(
        &field_chunks(&field.value_atoms, &field.span),
        abbreviations,
        is_list,
    )
    .map_err(|error| error.message)
}

#[expect(
    clippy::indexing_slicing,
    reason = "Graph/key lengths and every edge target are checked before traversal. The state vector has the same length and only checked vertices enter the stack."
)]
pub(crate) fn validate_graph(keys: &[&str], graph: &[Vec<usize>]) -> Result<(), String> {
    if keys.len() != graph.len() || graph.iter().flatten().any(|target| *target >= graph.len()) {
        return Err("reference graph contains an invalid vertex".into());
    }
    let mut state = vec![0u8; graph.len()];
    for start in 0..graph.len() {
        if state[start] != 0 {
            continue;
        }
        state[start] = 1;
        let mut pending = vec![(start, 0)];
        while let Some((current, next)) = pending.last_mut() {
            let Some(&target) = graph[*current].get(*next) else {
                state[*current] = 2;
                pending.pop();
                continue;
            };
            *next += 1;
            match state[target] {
                1 => {
                    return Err(format!(
                        "transformation creates a reference cycle involving citation key {:?}",
                        keys[target]
                    ));
                }
                0 => {
                    state[target] = 1;
                    pending.push((target, 0));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

pub(crate) fn encode_keys(keys: &[&str], is_list: bool) -> Result<String, String> {
    let plain = keys.join(", ");
    let round_trips = |value: &str| {
        let field = vec![Spanned::new(RawChunk::Normal(value), 0..value.len())];
        crate::library::normalize_reference(&field, &[], is_list)
            .is_ok_and(|actual| actual.iter().map(String::as_str).eq(keys.iter().copied()))
    };
    if round_trips(&plain) {
        return Ok(plain);
    }
    let encoded = keys
        .iter()
        .map(|key| {
            biblatex::Chunk::Verbatim((*key).to_string())
                .to_biblatex_string(false)
                .replace('-', "{-}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    if round_trips(&encoded) {
        Ok(encoded)
    } else {
        Err(format!(
            "citation keys {keys:?} cannot be represented faithfully in a reference field"
        ))
    }
}

pub(crate) fn field_chunks<'a>(
    atoms: &'a [crate::raw::RawValueAtom],
    span: &std::ops::Range<usize>,
) -> biblatex::Field<'a> {
    atoms
        .iter()
        .map(|atom| {
            let value = if atom.value_mode == crate::raw::RawValueMode::Bare
                && !atom.value.bytes().all(|c| c.is_ascii_digit())
            {
                RawChunk::Abbreviation(&atom.value)
            } else {
                RawChunk::Normal(&atom.value)
            };
            Spanned::new(value, span.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn malformed_graphs_fail_before_traversal() {
        assert!(super::validate_graph(&[], &[vec![]]).is_err());
        assert!(super::validate_graph(&["a"], &[vec![1]]).is_err());
    }
}
