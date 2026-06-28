use std::collections::{BTreeMap, HashMap, HashSet};

use crate::{RawEntryId, RawSyntaxDocument, RawSyntaxEntry};

use super::{DuplicateRule, MergeStrategy, TidyOptions, TidyWarning};

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

    pub fn entry<'a>(&'a self, entry: &'a RawSyntaxEntry) -> &'a RawSyntaxEntry {
        self.merged_entries.get(&entry.id).unwrap_or(entry)
    }

    fn retained_id(&self, id: RawEntryId) -> RawEntryId {
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

pub(crate) fn duplicate_plan(doc: &RawSyntaxDocument, options: &TidyOptions) -> DuplicatePlan {
    let Some(rules) = duplicate_rules(options) else {
        return DuplicatePlan::default();
    };
    let mut keys = BTreeMap::new();
    let mut dois = BTreeMap::new();
    let mut citations = BTreeMap::new();
    let mut abstracts = BTreeMap::new();
    let mut plan = DuplicatePlan::default();

    for entry in &doc.entries {
        for check in &rules {
            let duplicate = match check.rule {
                DuplicateRule::Key => duplicate_key(entry, &mut keys),
                DuplicateRule::Doi => duplicate_field(entry, "doi", &mut dois, None),
                DuplicateRule::Abstract => {
                    duplicate_field(entry, "abstract", &mut abstracts, Some(100))
                }
                DuplicateRule::Citation => duplicate_citation(entry, &mut citations),
            };
            if let Some(existing) = duplicate {
                let retained_id = plan.retained_id(existing.id);
                let retained_existing = entry_by_id(&doc.entries, retained_id).unwrap_or(existing);
                let existing_for_message = plan.entry(retained_existing);
                let merge_target = existing_for_message.clone();
                plan.warnings.push(TidyWarning::DuplicateEntry {
                    rule: check.rule,
                    message: duplicate_message(
                        check.rule,
                        check.do_merge,
                        entry,
                        existing_for_message,
                    ),
                });
                if check.do_merge {
                    if let Some(strategy) = options.merge {
                        plan.skip_entries.insert(entry.id);
                        plan.merge_targets.insert(entry.id, merge_target.id);
                        merge_entry(strategy, &mut plan.merged_entries, &merge_target, entry);
                    }
                }
            }
        }
    }

    plan
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

fn entry_by_id(entries: &[RawSyntaxEntry], id: RawEntryId) -> Option<&RawSyntaxEntry> {
    entries.iter().find(|entry| entry.id == id)
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
            target.key.clone_from(&duplicate.key);
            target.fields.clone_from(&duplicate.fields);
        }
        MergeStrategy::Combine | MergeStrategy::Overwrite => {
            for field in &duplicate.fields {
                let existing = target
                    .fields
                    .iter()
                    .position(|candidate| candidate.name.eq_ignore_ascii_case(&field.name));
                match (strategy, existing) {
                    (_, None) => target.fields.push(field.clone()),
                    (MergeStrategy::Overwrite, Some(index)) => {
                        target.fields[index].clone_from(field);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn duplicate_key<'a>(
    entry: &'a RawSyntaxEntry,
    keys: &mut BTreeMap<String, &'a RawSyntaxEntry>,
) -> Option<&'a RawSyntaxEntry> {
    if entry.key.is_empty() {
        return None;
    }
    let key = entry.key.to_ascii_lowercase();
    if let Some(existing) = keys.get(&key) {
        Some(*existing)
    } else {
        keys.insert(key, entry);
        None
    }
}

fn duplicate_field<'a>(
    entry: &'a RawSyntaxEntry,
    field: &str,
    values: &mut BTreeMap<String, &'a RawSyntaxEntry>,
    truncate: Option<usize>,
) -> Option<&'a RawSyntaxEntry> {
    let value = field_value(entry, field)?;
    let mut value = alpha_num(value);
    if let Some(limit) = truncate {
        value = value.chars().take(limit).collect();
    }
    if value.is_empty() {
        return None;
    }
    if let Some(existing) = values.get(&value) {
        Some(*existing)
    } else {
        values.insert(value, entry);
        None
    }
}

fn duplicate_citation<'a>(
    entry: &'a RawSyntaxEntry,
    citations: &mut BTreeMap<String, &'a RawSyntaxEntry>,
) -> Option<&'a RawSyntaxEntry> {
    let title = field_value(entry, "title")?;
    let author = field_value(entry, "author")?;
    let number = field_value(entry, "number").unwrap_or("0");
    let value = [
        alpha_num(&first_author_last(author)),
        alpha_num(title),
        alpha_num(number),
    ]
    .join(":");

    if let Some(existing) = citations.get(&value) {
        Some(*existing)
    } else {
        citations.insert(value, entry);
        None
    }
}

fn field_value<'a>(entry: &'a RawSyntaxEntry, field: &str) -> Option<&'a str> {
    entry
        .fields
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(field))
        .map(|field| field.value.as_str())
}

fn first_author_last(author: &str) -> String {
    let author = author.split(" and ").next().unwrap_or(author).trim();
    if let Some((last, _)) = author.split_once(',') {
        return last.trim().to_string();
    }

    let parts = author.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [] => String::new(),
        [last] => (*last).to_string(),
        [_, last @ ..] => last.join(" "),
    }
}

fn alpha_num(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
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
            "Duplicate {action}. Entry {} has an identical DOI to entry {}.",
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
