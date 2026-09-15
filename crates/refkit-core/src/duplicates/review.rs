use super::{
    DuplicateConflict, DuplicateConflictKind, DuplicateEvidence, DuplicateGroup, DuplicateMember,
    DuplicateReport, DuplicateValue, MergeError, MergeErrorCode, signature,
};
use crate::{DuplicateRule, RawDocument, RawEntryId, raw::RawSyntaxEntry};
use std::collections::{BTreeMap, BTreeSet, HashSet};

impl RawDocument {
    /// Inspect deterministic duplicate groups using selected rules or all default rules.
    ///
    /// # Errors
    /// Rejects source rendering failures or input exceeding the review resource budget.
    pub fn find_duplicates(
        &self,
        rules: Option<&[DuplicateRule]>,
    ) -> Result<DuplicateReport, MergeError> {
        let rules = rules.unwrap_or(&[
            DuplicateRule::Doi,
            DuplicateRule::Key,
            DuplicateRule::Abstract,
            DuplicateRule::Citation,
        ]);
        let source = self
            .render()
            .map_err(|message| MergeError::new(MergeErrorCode::InvalidSelection, message))?;
        crate::library::validate_source(&source)
            .map_err(|error| MergeError::new(MergeErrorCode::ResourceLimit, error.message))?;
        let syntax = self.clone().into_syntax();
        let mut seen = HashSet::new();
        let rules = rules
            .iter()
            .copied()
            .filter(|rule| seen.insert(*rule))
            .collect::<Vec<_>>();
        let mut components = (0..syntax.entries.len()).collect::<Vec<_>>();
        let mut evidence = Vec::new();
        for rule in &rules {
            add_rule_evidence(&syntax.entries, *rule, &mut components, &mut evidence);
        }
        let mut groups: BTreeMap<usize, Vec<&RawSyntaxEntry>> = BTreeMap::new();
        for entry in &syntax.entries {
            groups
                .entry(root(&mut components, entry.id.index()))
                .or_default()
                .push(entry);
        }
        let mut grouped_evidence: BTreeMap<usize, Vec<DuplicateEvidence>> = BTreeMap::new();
        for evidence in evidence {
            let Some(first) = evidence.members.first() else {
                continue;
            };
            grouped_evidence
                .entry(root(&mut components, first.index()))
                .or_default()
                .push(evidence);
        }
        let groups = groups
            .into_iter()
            .filter_map(|(id, entries)| {
                let [first, _, ..] = entries.as_slice() else {
                    return None;
                };
                Some(DuplicateGroup {
                    id: first.id,
                    members: entries
                        .iter()
                        .map(|entry| DuplicateMember {
                            entry_id: entry.id,
                            key: entry.key.clone(),
                        })
                        .collect(),
                    evidence: grouped_evidence.remove(&id).unwrap_or_default(),
                    conflicts: conflicts(&entries, &source),
                })
            })
            .collect();
        Ok(DuplicateReport { rules, groups })
    }
}

fn add_rule_evidence(
    entries: &[RawSyntaxEntry],
    rule: DuplicateRule,
    components: &mut [usize],
    evidence: &mut Vec<DuplicateEvidence>,
) {
    let mut buckets: BTreeMap<String, Vec<RawEntryId>> = BTreeMap::new();
    for entry in entries {
        if let Some(value) = signature(entry, rule) {
            buckets.entry(value).or_default().push(entry.id);
        }
    }
    for (signature, members) in buckets {
        let [first, _, ..] = members.as_slice() else {
            continue;
        };
        for member in members.iter().skip(1) {
            join(components, first.index(), member.index());
        }
        evidence.push(DuplicateEvidence {
            rule,
            signature,
            members,
        });
    }
}

#[expect(
    clippy::indexing_slicing,
    reason = "Components are initialized from source-order entry indices. Union only stores roots from that same slice, so parent and grandparent indices remain in bounds."
)]
fn root(parents: &mut [usize], mut id: usize) -> usize {
    while parents[id] != id {
        parents[id] = parents[parents[id]];
        id = parents[id];
    }
    id
}

#[expect(
    clippy::indexing_slicing,
    reason = "Both operands are roots returned from the same component slice. Linking the larger root to the smaller preserves valid, acyclic parent indices."
)]
fn join(parents: &mut [usize], left: usize, right: usize) {
    let left = root(parents, left);
    let right = root(parents, right);
    parents[left.max(right)] = left.min(right);
}

pub(super) fn values(
    entries: &[&RawSyntaxEntry],
    source: &str,
) -> BTreeMap<String, Vec<DuplicateValue>> {
    let mut values: BTreeMap<String, Vec<DuplicateValue>> = BTreeMap::new();
    for entry in entries {
        for field in &entry.fields {
            values
                .entry(field.name.to_ascii_lowercase())
                .or_default()
                .push(DuplicateValue {
                    entry_id: entry.id,
                    field_id: Some(field.id),
                    value: field.value.clone(),
                    expression: source[field.patch_span.clone()].into(),
                });
        }
    }
    values
}

pub(super) fn conflicts(entries: &[&RawSyntaxEntry], source: &str) -> Vec<DuplicateConflict> {
    let mut conflicts = Vec::new();
    if entries
        .iter()
        .map(|entry| entry.kind.to_ascii_lowercase())
        .collect::<BTreeSet<_>>()
        .len()
        > 1
    {
        conflicts.push(DuplicateConflict {
            kind: DuplicateConflictKind::EntryType,
            field: "@type".into(),
            values: entries
                .iter()
                .map(|entry| DuplicateValue {
                    entry_id: entry.id,
                    field_id: None,
                    value: entry.kind.clone(),
                    expression: entry.kind.clone(),
                })
                .collect(),
        });
    }
    for (field, values) in values(entries, source) {
        if values
            .iter()
            .map(|value| &value.expression)
            .collect::<BTreeSet<_>>()
            .len()
            <= 1
        {
            continue;
        }
        let identifiers = values
            .iter()
            .filter_map(|value| {
                crate::validation::canonical_identifier(&field, &value.value).and_then(Result::ok)
            })
            .collect::<BTreeSet<_>>();
        let kind = if identifiers.len() > 1 {
            DuplicateConflictKind::Identifier
        } else {
            DuplicateConflictKind::Field
        };
        conflicts.push(DuplicateConflict {
            kind,
            field,
            values,
        });
    }
    conflicts
}
