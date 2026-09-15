use std::collections::{BTreeMap, HashMap, HashSet};

use crate::raw::{RawEntryId, RawSyntaxDocument, RawSyntaxEntry};

use super::{DuplicateRule, MergeStrategy, TidyError, TidyOptions, TidyWarning};

#[derive(Debug, Default, Clone)]
pub(crate) struct DuplicatePlan {
    pub warnings: Vec<TidyWarning>,
    skip_entries: HashSet<RawEntryId>,
    merged_entries: HashMap<RawEntryId, RawSyntaxEntry>,
    merge_targets: HashMap<RawEntryId, RawEntryId>,
}

impl DuplicatePlan {
    pub fn should_skip(&self, id: RawEntryId) -> bool {
        self.skip_entries.contains(&id)
    }

    pub fn apply(&mut self, doc: &mut RawSyntaxDocument) -> Result<(), TidyError> {
        for (id, entry) in self.merged_entries.drain() {
            let target = doc.entries.get_mut(id.index()).ok_or_else(|| {
                TidyError::Reference("duplicate plan refers to a missing entry".to_string())
            })?;
            *target = entry;
        }
        Ok(())
    }

    pub fn retained_id(&self, id: RawEntryId) -> RawEntryId {
        let mut current = id;
        while let Some(next) = self.merge_targets.get(&current).copied() {
            if next == current {
                break;
            }
            current = next;
        }
        current
    }
}

#[derive(Debug, Clone, Copy)]
struct DuplicateCheckRule {
    rule: DuplicateRule,
    do_merge: bool,
}

pub(crate) fn duplicate_plan(
    doc: &RawSyntaxDocument,
    options: &TidyOptions,
) -> Result<DuplicatePlan, TidyError> {
    let Some(rules) = duplicate_rules(options) else {
        return Ok(DuplicatePlan::default());
    };
    let mut keys = BTreeMap::new();
    let mut dois = BTreeMap::new();
    let mut citations = BTreeMap::new();
    let mut abstracts = BTreeMap::new();
    let mut plan = DuplicatePlan::default();

    for entry in &doc.entries {
        for check in &rules {
            let duplicate = match check.rule {
                DuplicateRule::Key => duplicate_match(entry, check.rule, &mut keys),
                DuplicateRule::Doi => duplicate_match(entry, check.rule, &mut dois),
                DuplicateRule::Abstract => duplicate_match(entry, check.rule, &mut abstracts),
                DuplicateRule::Citation => duplicate_match(entry, check.rule, &mut citations),
            };
            let Some(existing) = duplicate else {
                continue;
            };
            plan.warnings.push(TidyWarning::DuplicateEntry {
                rule: check.rule,
                message: duplicate_message(check.rule, check.do_merge, entry, existing),
            });
            if !check.do_merge || options.merge.is_none() {
                continue;
            }
            let left = plan.retained_id(existing.id);
            let right = plan.retained_id(entry.id);
            if left == right {
                continue;
            }
            let (target, source) = if left.index() < right.index() {
                (left, right)
            } else {
                (right, left)
            };
            plan.merge_targets.insert(source, target);
        }
    }

    let Some(strategy) = options.merge else {
        return Ok(plan);
    };
    for entry in &doc.entries {
        let target_id = plan.retained_id(entry.id);
        if entry.id != target_id {
            plan.skip_entries.insert(entry.id);
            let target = doc.entries.get(target_id.index()).ok_or_else(|| {
                TidyError::Reference(
                    "duplicate plan refers to a missing retained entry".to_string(),
                )
            })?;
            merge_entry(strategy, &mut plan.merged_entries, target, entry);
        }
    }

    Ok(plan)
}

fn duplicate_rules(options: &TidyOptions) -> Option<Vec<DuplicateCheckRule>> {
    if options.duplicates.is_none() && options.merge.is_none() {
        return None;
    }

    let mut rules = Vec::new();
    let mut seen = HashSet::new();

    if let Some(duplicates) = &options.duplicates {
        for rule in duplicates {
            if seen.insert(*rule) {
                rules.push(DuplicateCheckRule {
                    rule: *rule,
                    do_merge: options.merge.is_some(),
                });
            }
        }
    } else if options.merge.is_some() {
        for rule in [
            DuplicateRule::Doi,
            DuplicateRule::Citation,
            DuplicateRule::Abstract,
        ] {
            seen.insert(rule);
            rules.push(DuplicateCheckRule {
                rule,
                do_merge: true,
            });
        }
    }

    if !seen.contains(&DuplicateRule::Key) {
        rules.push(DuplicateCheckRule {
            rule: DuplicateRule::Key,
            do_merge: false,
        });
    }
    Some(rules)
}

fn merge_entry(
    strategy: MergeStrategy,
    merged_entries: &mut HashMap<RawEntryId, RawSyntaxEntry>,
    target: &RawSyntaxEntry,
    duplicate: &RawSyntaxEntry,
) {
    let target = merged_entries
        .entry(target.id)
        .or_insert_with(|| target.clone());
    match strategy {
        MergeStrategy::First => {}
        MergeStrategy::Last => {
            let id = target.id;
            target.clone_from(duplicate);
            target.id = id;
        }
        MergeStrategy::Combine | MergeStrategy::Overwrite => {
            for field in &duplicate.fields {
                let existing = target
                    .fields
                    .iter_mut()
                    .find(|candidate| candidate.name.eq_ignore_ascii_case(&field.name));
                match (strategy, existing) {
                    (_, None) => target.fields.push(field.clone()),
                    (MergeStrategy::Overwrite, Some(existing)) => {
                        existing.clone_from(field);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn duplicate_match<'a>(
    entry: &'a RawSyntaxEntry,
    rule: DuplicateRule,
    values: &mut BTreeMap<String, &'a RawSyntaxEntry>,
) -> Option<&'a RawSyntaxEntry> {
    let signature = crate::duplicates::signature(entry, rule)?;
    if let Some(existing) = values.get(&signature) {
        Some(*existing)
    } else {
        values.insert(signature, entry);
        None
    }
}

fn duplicate_message(
    rule: DuplicateRule,
    do_merge: bool,
    entry: &RawSyntaxEntry,
    existing: &RawSyntaxEntry,
) -> String {
    let action = if do_merge { "removed" } else { "detected" };
    match rule {
        DuplicateRule::Key => format!(
            "Duplicate {action}. The citation key {} has already been used.",
            entry.key
        ),
        DuplicateRule::Doi => format!(
            "Duplicate {action}. Entry {} has the same DOI signature as entry {}.",
            entry.key, existing.key
        ),
        DuplicateRule::Citation => format!(
            "Duplicate {action}. Entry {} has similar content to entry {}.",
            entry.key, existing.key
        ),
        DuplicateRule::Abstract => format!(
            "Duplicate {action}. Entry {} has a similar abstract to entry {}.",
            entry.key, existing.key
        ),
    }
}
