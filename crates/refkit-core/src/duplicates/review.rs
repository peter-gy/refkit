use super::{
    Components, DuplicateConflict, DuplicateConflictKind, DuplicateEvidence, DuplicateGroup,
    DuplicateMember, DuplicateReport, DuplicateValue, EntryView, MergeError, MergeErrorCode,
    signature,
};
use crate::{DuplicateRule, RawDocument, RawEntryId};
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
        let entries = self
            .syntax_data()
            .entry_blocks
            .iter()
            .enumerate()
            .map(|(index, entry)| EntryView::new(index, entry))
            .collect::<Vec<_>>();
        let mut seen = HashSet::new();
        let rules = rules
            .iter()
            .copied()
            .filter(|rule| seen.insert(*rule))
            .collect::<Vec<_>>();
        let mut components = Components::new(entries.iter().map(|entry| entry.id));
        let mut evidence = Vec::new();
        for rule in &rules {
            add_rule_evidence(&entries, *rule, &mut components, &mut evidence);
        }
        let mut grouped_evidence: BTreeMap<usize, Vec<DuplicateEvidence>> = BTreeMap::new();
        for evidence in evidence {
            let Some(first) = evidence.members.first() else {
                continue;
            };
            grouped_evidence
                .entry(components.root(*first).index())
                .or_default()
                .push(evidence);
        }
        let mut groups: BTreeMap<usize, Vec<&EntryView<'_>>> = BTreeMap::new();
        for (entry, root) in entries.iter().zip(components.freeze()) {
            groups.entry(root.index()).or_default().push(entry);
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
                            key: entry.key.to_string(),
                        })
                        .collect(),
                    evidence: grouped_evidence.remove(&id).unwrap_or_default(),
                    conflicts: conflicts(&entries, &values(&entries, &source)),
                })
            })
            .collect();
        Ok(DuplicateReport { rules, groups })
    }
}

fn add_rule_evidence(
    entries: &[EntryView<'_>],
    rule: DuplicateRule,
    components: &mut Components,
    evidence: &mut Vec<DuplicateEvidence>,
) {
    let mut buckets: BTreeMap<String, Vec<RawEntryId>> = BTreeMap::new();
    for entry in entries {
        if let Some(value) = signature(entry.key, |name| entry.field(name), rule) {
            buckets.entry(value).or_default().push(entry.id);
        }
    }
    for (signature, members) in buckets {
        let [first, _, ..] = members.as_slice() else {
            continue;
        };
        for member in members.iter().skip(1) {
            components.join(*first, *member);
        }
        evidence.push(DuplicateEvidence {
            rule,
            signature,
            members,
        });
    }
}

pub(super) fn values(
    entries: &[&EntryView<'_>],
    source: &str,
) -> BTreeMap<String, Vec<DuplicateValue>> {
    let mut values: BTreeMap<String, Vec<DuplicateValue>> = BTreeMap::new();
    for entry in entries {
        for (field_id, field) in entry.fields() {
            values
                .entry(field.name.to_ascii_lowercase())
                .or_default()
                .push(DuplicateValue {
                    entry_id: entry.id,
                    field_id: Some(field_id),
                    value: field.value.clone(),
                    expression: source[field.patch_span.clone()].into(),
                });
        }
    }
    values
}

pub(super) fn conflicts(
    entries: &[&EntryView<'_>],
    fields: &BTreeMap<String, Vec<DuplicateValue>>,
) -> Vec<DuplicateConflict> {
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
                    value: entry.kind.to_string(),
                    expression: entry.kind.to_string(),
                })
                .collect(),
        });
    }
    for (field, values) in fields {
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
                crate::validation::canonical_identifier(field, &value.value).and_then(Result::ok)
            })
            .collect::<BTreeSet<_>>();
        let kind = if identifiers.len() > 1 {
            DuplicateConflictKind::Identifier
        } else {
            DuplicateConflictKind::Field
        };
        conflicts.push(DuplicateConflict {
            kind,
            field: field.clone(),
            values: values.clone(),
        });
    }
    conflicts
}
