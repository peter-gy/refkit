//! Conformance with pinned bibliography formatter specification fixtures.

#![cfg(test)]

use std::fs;
use std::path::PathBuf;

use refkit_core::{TidyOptions, tidy_bibtex as tidy};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SpecDocument {
    #[serde(default)]
    options: SpecOptions,
    input: String,
    expected: Option<String>,
    #[serde(default)]
    warnings: Vec<SpecWarning>,
}

#[derive(Debug, Default, Deserialize)]
struct SpecWarning {
    rule: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct SpecOptions {
    align: Option<serde_yaml::Value>,
    #[serde(rename = "blankLines")]
    blank_lines: Option<bool>,
    curly: Option<bool>,
    duplicates: Option<serde_yaml::Value>,
    #[serde(rename = "dropAllCaps")]
    drop_all_caps: Option<bool>,
    #[serde(rename = "enclosingBraces")]
    enclosing_braces: Option<serde_yaml::Value>,
    #[serde(rename = "encodeUrls")]
    encode_urls: Option<bool>,
    escape: Option<bool>,
    #[serde(rename = "generateKeys")]
    generate_keys: Option<serde_yaml::Value>,
    lowercase: Option<bool>,
    #[serde(rename = "maxAuthors")]
    max_authors: Option<usize>,
    merge: Option<serde_yaml::Value>,
    months: Option<bool>,
    numeric: Option<bool>,
    omit: Option<Vec<String>>,
    #[serde(rename = "removeBraces")]
    remove_braces: Option<serde_yaml::Value>,
    #[serde(rename = "removeEmptyFields")]
    remove_empty_fields: Option<bool>,
    #[serde(rename = "removeDuplicateFields")]
    remove_duplicate_fields: Option<bool>,
    space: Option<serde_yaml::Value>,
    sort: Option<serde_yaml::Value>,
    #[serde(rename = "sortFields")]
    sort_fields: Option<serde_yaml::Value>,
    #[serde(rename = "stripComments")]
    strip_comments: Option<bool>,
    #[serde(rename = "stripEnclosingBraces")]
    strip_enclosing_braces: Option<bool>,
    tab: Option<bool>,
    #[serde(rename = "tidyComments")]
    tidy_comments: Option<bool>,
    #[serde(rename = "trailingCommas")]
    trailing_commas: Option<bool>,
    wrap: Option<usize>,
}

#[test]
fn extended_name_format_retains_the_entry() {
    for spec in read_specs("extended-name-format.spec.yaml") {
        let result = tidy(&spec.input, spec.options.into_tidy_options()).unwrap();
        assert_eq!(result.count, 1);
        assert_eq!(
            refkit_core::RawDocument::parse(&result.bibtex).entry_keys(),
            ["LDN3"]
        );
    }
}

#[test]
fn all_upstream_specs_match_upstream() {
    let mut failures = Vec::new();
    for name in all_spec_files() {
        for (index, spec) in read_specs(&name).into_iter().enumerate() {
            check_spec(spec, &name, index, &mut failures);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

impl SpecOptions {
    fn into_tidy_options(self) -> TidyOptions {
        let mut options = TidyOptions::default();
        if let Some(align) = self.align {
            match align {
                serde_yaml::Value::Bool(false) => options.align = None,
                serde_yaml::Value::Bool(true) => options.align = Some(14),
                serde_yaml::Value::Number(value) => {
                    options.align = value.as_u64().map(|value| usize::try_from(value).unwrap());
                }
                _ => {}
            }
        }
        options.blank_lines = self.blank_lines.unwrap_or(options.blank_lines);
        options.curly = self.curly.unwrap_or(options.curly);
        if let Some(duplicates) = self.duplicates {
            options.duplicates = duplicate_rules(duplicates);
        }
        options.drop_all_caps = self.drop_all_caps.unwrap_or(options.drop_all_caps);
        options.enclosing_braces = string_option(
            self.enclosing_braces,
            options.enclosing_braces,
            TidyOptions::default()
                .with_enclosing_braces()
                .enclosing_braces,
        );
        options.encode_urls = self.encode_urls.unwrap_or(options.encode_urls);
        options.escape = self.escape.unwrap_or(options.escape);
        if let Some(generate_keys) = self.generate_keys {
            match generate_keys {
                serde_yaml::Value::Bool(true) => options = options.with_generate_keys(),
                serde_yaml::Value::Bool(false) => options.generate_keys = None,
                serde_yaml::Value::String(value) => options.generate_keys = Some(value),
                _ => {}
            }
        }
        options.lowercase = self.lowercase.unwrap_or(options.lowercase);
        if let Some(max_authors) = self.max_authors {
            options.max_authors = Some(max_authors);
        }
        if let Some(merge) = self.merge {
            match merge {
                serde_yaml::Value::Bool(true) => {
                    options.merge = Some(refkit_core::MergeStrategy::Combine);
                }
                serde_yaml::Value::Bool(false) => options.merge = None,
                serde_yaml::Value::String(value) => {
                    options.merge = match value.as_str() {
                        "first" => Some(refkit_core::MergeStrategy::First),
                        "last" => Some(refkit_core::MergeStrategy::Last),
                        "combine" => Some(refkit_core::MergeStrategy::Combine),
                        "overwrite" => Some(refkit_core::MergeStrategy::Overwrite),
                        _ => None,
                    };
                }
                _ => {}
            }
        }
        options.months = self.months.unwrap_or(options.months);
        options.numeric = self.numeric.unwrap_or(options.numeric);
        if let Some(omit) = self.omit {
            options.omit = omit;
        }
        options.remove_braces = string_option(
            self.remove_braces,
            options.remove_braces,
            TidyOptions::default().with_remove_braces().remove_braces,
        );
        options.remove_duplicate_fields = self
            .remove_duplicate_fields
            .unwrap_or(options.remove_duplicate_fields);
        options.remove_empty_fields = self
            .remove_empty_fields
            .unwrap_or(options.remove_empty_fields);
        apply_space(self.space, &mut options.space);
        options.sort_fields = string_option(
            self.sort_fields,
            options.sort_fields,
            TidyOptions::default().with_sort_fields().sort_fields,
        );
        options.sort = string_option(
            self.sort,
            options.sort,
            TidyOptions::default().with_sort().sort,
        );
        options.strip_comments = self.strip_comments.unwrap_or(options.strip_comments);
        options.strip_enclosing_braces = self
            .strip_enclosing_braces
            .unwrap_or(options.strip_enclosing_braces);
        options.tab = self.tab.unwrap_or(options.tab);
        options.tidy_comments = self.tidy_comments.unwrap_or(options.tidy_comments);
        options.trailing_commas = self.trailing_commas.unwrap_or(options.trailing_commas);
        if let Some(wrap) = self.wrap {
            options.wrap = Some(wrap);
        }
        options
    }
}

fn string_sequence(values: Vec<serde_yaml::Value>) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect()
}

fn duplicate_rules(value: serde_yaml::Value) -> Option<Vec<refkit_core::DuplicateRule>> {
    match value {
        serde_yaml::Value::Bool(true) => Some(vec![
            refkit_core::DuplicateRule::Doi,
            refkit_core::DuplicateRule::Citation,
            refkit_core::DuplicateRule::Abstract,
            refkit_core::DuplicateRule::Key,
        ]),
        serde_yaml::Value::Sequence(values) => Some(
            values
                .into_iter()
                .filter_map(|value| {
                    let value = value.as_str()?;
                    match value {
                        "doi" => Some(refkit_core::DuplicateRule::Doi),
                        "key" => Some(refkit_core::DuplicateRule::Key),
                        "abstract" => Some(refkit_core::DuplicateRule::Abstract),
                        "citation" => Some(refkit_core::DuplicateRule::Citation),
                        _ => None,
                    }
                })
                .collect(),
        ),
        _ => None,
    }
}

fn snippet(value: &str) -> String {
    const LIMIT: usize = 2_000;
    let escaped = format!("{value:?}");
    let mut out = escaped.chars().take(LIMIT).collect::<String>();
    if escaped.chars().count() > LIMIT {
        out.push_str("\n...");
    }
    out
}

fn first_diff(expected: &str, actual: &str) -> String {
    let index = expected
        .char_indices()
        .zip(actual.char_indices())
        .find_map(|((expected_index, expected_ch), (_, actual_ch))| {
            if expected_ch == actual_ch {
                None
            } else {
                Some(expected_index)
            }
        })
        .unwrap_or_else(|| expected.len().min(actual.len()));
    format!(
        "first difference at byte {index}\nexpected around: {}\nactual around: {}",
        around(expected, index),
        around(actual, index)
    )
}

fn around(value: &str, index: usize) -> String {
    let start = value[..index]
        .char_indices()
        .rev()
        .nth(160)
        .map_or(0, |(idx, _)| idx);
    let end = value[index..]
        .char_indices()
        .nth(160)
        .map_or(value.len(), |(idx, _)| index + idx);
    format!("{:?}", &value[start..end])
}

fn all_spec_files() -> Vec<String> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../testdata/tidy/spec");
    let mut files = fs::read_dir(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|error| panic!("failed to read spec entry: {error}"))
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .filter(|name| name.ends_with(".spec.yaml"))
        .collect::<Vec<_>>();
    files.sort();
    files
}

fn read_specs(name: &str) -> Vec<SpecDocument> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../testdata/tidy/spec");
    path.push(name);
    let mut text = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("failed to read {}: {error}", path.display());
    });
    if name == "spacing-before-first-entry.spec.yaml" {
        text = text
            .replace("input: |", "input: |2")
            .replace("expected: |", "expected: |2");
    }
    if text.contains("\n...\n") && !text.contains("\n---\n") {
        let chunks = text.split("\n...\n").collect::<Vec<_>>();
        let last = chunks.len().saturating_sub(1);
        return chunks
            .into_iter()
            .enumerate()
            .filter(|(_, document)| !document.trim().is_empty())
            .map(|(index, document)| {
                let document = if index < last {
                    format!("{document}\n")
                } else {
                    document.to_string()
                };
                parse_spec_document(&document, &path)
            })
            .collect();
    }
    serde_yaml::Deserializer::from_str(&text)
        .filter_map(|document| {
            let value = serde_yaml::Value::deserialize(document).unwrap_or_else(|error| {
                panic!("failed to parse {}: {error}", path.display());
            });
            if value.is_null() {
                return None;
            }
            Some(serde_yaml::from_value(value).unwrap_or_else(|error| {
                panic!("failed to parse {}: {error}", path.display());
            }))
        })
        .collect()
}

fn parse_spec_document(text: &str, path: &std::path::Path) -> SpecDocument {
    serde_yaml::from_str(text).unwrap_or_else(|error| {
        panic!("failed to parse {}: {error}", path.display());
    })
}

fn check_spec(spec: SpecDocument, name: &str, index: usize, failures: &mut Vec<String>) {
    match tidy(&spec.input, spec.options.into_tidy_options()) {
        Ok(result) => {
            if let Some(expected) = spec.expected.as_ref()
                && result.bibtex != *expected
            {
                failures.push(format!(
                    "{name}#{index}: output mismatch\n{}\nexpected:\n{}\nactual:\n{}",
                    first_diff(expected, &result.bibtex),
                    snippet(expected),
                    snippet(&result.bibtex)
                ));
            }
            if !spec.warnings.is_empty() {
                let expected_rules = spec
                    .warnings
                    .iter()
                    .filter_map(|warning| warning.rule.as_deref())
                    .collect::<Vec<_>>();
                let actual_rules = result
                    .warnings
                    .iter()
                    .filter_map(|warning| warning.rule().map(refkit_core::DuplicateRule::as_str))
                    .collect::<Vec<_>>();
                if result.warnings.len() != spec.warnings.len()
                    || (!expected_rules.is_empty() && actual_rules != expected_rules)
                {
                    failures.push(format!("{name}#{index}: warning mismatch"));
                }
            }
        }
        Err(error) => failures.push(format!("{name}#{index}: {error}")),
    }
}

fn apply_space(space: Option<serde_yaml::Value>, current: &mut usize) {
    if let Some(space) = space {
        match space {
            serde_yaml::Value::Bool(true) => *current = 2,
            serde_yaml::Value::Number(value) => {
                if let Some(value) = value.as_u64() {
                    *current = usize::try_from(value).unwrap();
                }
            }
            _ => {}
        }
    }
}

fn string_option(
    value: Option<serde_yaml::Value>,
    default: Option<Vec<String>>,
    enabled: Option<Vec<String>>,
) -> Option<Vec<String>> {
    match value {
        Some(serde_yaml::Value::Bool(true)) => enabled,
        Some(serde_yaml::Value::Bool(false)) => None,
        Some(serde_yaml::Value::Sequence(values)) => Some(string_sequence(values)),
        _ => default,
    }
}
