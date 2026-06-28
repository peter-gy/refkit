use std::fs;
use std::path::PathBuf;

use bibtex_tidy_rs::{DuplicateRule, TidyOptions, tidy};
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
fn defaults_spec_matches_upstream_fixture() {
    let spec = read_spec("defaults.spec.yaml");
    let result = tidy(&spec.input, spec.options.clone().into_tidy_options()).unwrap();

    assert_expected("defaults.spec.yaml", &spec, &result);
}

#[test]
fn align_off_spec_matches_upstream_fixture() {
    let spec = read_spec("align-off.spec.yaml");
    let result = tidy(&spec.input, spec.options.clone().into_tidy_options()).unwrap();

    assert_expected("align-off.spec.yaml", &spec, &result);
}

#[test]
fn option_smoke_specs_match_upstream_fixtures() {
    for name in [
        "indent-spaces.spec.yaml",
        "indent-spaces-custom.spec.yaml",
        "indent-tab.spec.yaml",
        "align20.spec.yaml",
        "at-symbol-in-comment.spec.yaml",
        "at-symbol-in-title.spec.yaml",
        "big-numbers.spec.yaml",
        "concatenation.spec.yaml",
        "escape-off.spec.yaml",
        "escape.spec.yaml",
        "no-lowercase.spec.yaml",
        "empty-entry.spec.yaml",
        "empty-key.spec.yaml",
        "curly.spec.yaml",
        "encode-urls.spec.yaml",
        "encode-urls-off.spec.yaml",
        "drop-all-caps.spec.yaml",
        "leading-commas.spec.yaml",
        "max-authors.spec.yaml",
        "multiline-field.spec.yaml",
        "numeric.spec.yaml",
        "omit-properties.spec.yaml",
        "one-line-bib.spec.yaml",
        "enclosing-braces.spec.yaml",
        "enclosing-braces-custom-fields.spec.yaml",
        "enclosing-braces-in-text.spec.yaml",
        "enclosing-braces-around-command.spec.yaml",
        "enclosing-braces-with-escape.spec.yaml",
        "remove-braces.spec.yaml",
        "remove-braces-specific-fields.spec.yaml",
        "strip-double-brace.spec.yaml",
        "remove-duplicate-fields.spec.yaml",
        "remove-duplicate-fields-off.spec.yaml",
        "remove-empty-fields.spec.yaml",
        "remove-empty-fields-off.spec.yaml",
        "sort-fields.spec.yaml",
        "sort-fields-custom.spec.yaml",
        "sort-key.spec.yaml",
        "sort-descending.spec.yaml",
        "sort-multi-key.spec.yaml",
        "sort-numeric.spec.yaml",
        "sort-special.spec.yaml",
        "sort.spec.yaml",
        "spacing-before-first-entry.spec.yaml",
        "trailing-commas.spec.yaml",
        "paragraph.spec.yaml",
        "wrap.spec.yaml",
    ] {
        let spec = read_spec(name);
        let result = tidy(&spec.input, spec.options.clone().into_tidy_options())
            .unwrap_or_else(|error| panic!("{name}: {error}"));

        assert_expected(name, &spec, &result);
    }
}

#[test]
fn abbreviate_months_spec_documents_match_upstream_fixture() {
    for spec in read_specs("abbreviate-months.spec.yaml") {
        let result = tidy(&spec.input, spec.options.clone().into_tidy_options()).unwrap();
        assert_expected("abbreviate-months.spec.yaml", &spec, &result);
    }
}

#[test]
fn generate_keys_spec_documents_match_upstream_fixture() {
    for spec in read_specs("generate-keys.spec.yaml") {
        let result = tidy(&spec.input, spec.options.clone().into_tidy_options()).unwrap();
        assert_expected("generate-keys.spec.yaml", &spec, &result);
    }
}

#[test]
fn duplicate_warning_specs_match_upstream_fixtures() {
    for (name, rules) in [
        (
            "duplicate-abstracts.spec.yaml",
            vec![DuplicateRule::Abstract],
        ),
        (
            "duplicate-citations.spec.yaml",
            vec![DuplicateRule::Citation, DuplicateRule::Citation],
        ),
        ("duplicate-dois.spec.yaml", vec![DuplicateRule::Doi]),
        ("duplicate-keys.spec.yaml", vec![DuplicateRule::Key]),
    ] {
        let spec = read_spec(name);
        let result = tidy(&spec.input, spec.options.clone().into_tidy_options())
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let actual = result
            .warnings
            .iter()
            .filter_map(|warning| warning.rule())
            .collect::<Vec<_>>();

        assert_eq!(actual, rules, "{name}");
    }
}

#[test]
fn duplicate_merge_specs_match_upstream_fixtures() {
    for name in [
        "duplicate-merge-1.spec.yaml",
        "duplicate-merge-2.spec.yaml",
        "duplicate-merge-combine.spec.yaml",
        "duplicate-merge-first.spec.yaml",
        "duplicate-merge-keys.spec.yaml",
        "duplicate-merge-last.spec.yaml",
        "duplicate-merge-overwrite.spec.yaml",
    ] {
        let spec = read_spec(name);
        let result = tidy(&spec.input, spec.options.clone().into_tidy_options())
            .unwrap_or_else(|error| panic!("{name}: {error}"));

        assert_expected(name, &spec, &result);
        if !spec.warnings.is_empty() {
            assert_eq!(result.warnings.len(), spec.warnings.len(), "{name}");
        }
    }
}

#[test]
fn extended_name_format_spec_parses_upstream_fixture() {
    let spec = read_spec("extended-name-format.spec.yaml");
    let result = tidy(&spec.input, spec.options.clone().into_tidy_options()).unwrap();

    assert_eq!(result.count, 1);
    assert!(result.warnings.is_empty());
}

#[test]
fn vendored_spec_inventory_matches_upstream_count() {
    assert_eq!(all_spec_files().len(), 67);
    let document_count = all_spec_files()
        .iter()
        .flat_map(|path| read_specs(path))
        .count();
    assert_eq!(document_count, 78);
}

#[test]
fn all_upstream_specs_match_upstream() {
    let mut failures = Vec::new();
    for name in all_spec_files() {
        for (index, spec) in read_specs(&name).into_iter().enumerate() {
            match tidy(&spec.input, spec.options.clone().into_tidy_options()) {
                Ok(result) => {
                    if let Some(expected) = spec.expected.as_ref() {
                        if result.bibtex != *expected {
                            failures.push(format!(
                                "{name}#{index}: output mismatch\n{}\nexpected:\n{}\nactual:\n{}",
                                first_diff(expected, &result.bibtex),
                                snippet(expected),
                                snippet(&result.bibtex)
                            ));
                        }
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
                            .filter_map(|warning| warning.rule().map(|rule| rule.as_str()))
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
                    options.align = value.as_u64().map(|value| value as usize);
                }
                _ => {}
            }
        }
        if let Some(blank_lines) = self.blank_lines {
            options.blank_lines = blank_lines;
        }
        if let Some(curly) = self.curly {
            options.curly = curly;
        }
        if let Some(duplicates) = self.duplicates {
            options.duplicates = duplicate_rules(duplicates);
        }
        if let Some(drop_all_caps) = self.drop_all_caps {
            options.drop_all_caps = drop_all_caps;
        }
        if let Some(enclosing_braces) = self.enclosing_braces {
            match enclosing_braces {
                serde_yaml::Value::Bool(true) => options = options.with_enclosing_braces(),
                serde_yaml::Value::Bool(false) => options.enclosing_braces = None,
                serde_yaml::Value::Sequence(values) => {
                    options.enclosing_braces = Some(string_sequence(values));
                }
                _ => {}
            }
        }
        if let Some(encode_urls) = self.encode_urls {
            options.encode_urls = encode_urls;
        }
        if let Some(escape) = self.escape {
            options.escape = escape;
        }
        if let Some(generate_keys) = self.generate_keys {
            match generate_keys {
                serde_yaml::Value::Bool(true) => options = options.with_generate_keys(),
                serde_yaml::Value::Bool(false) => options.generate_keys = None,
                serde_yaml::Value::String(value) => options.generate_keys = Some(value),
                _ => {}
            }
        }
        if let Some(lowercase) = self.lowercase {
            options.lowercase = lowercase;
        }
        if let Some(max_authors) = self.max_authors {
            options.max_authors = Some(max_authors);
        }
        if let Some(merge) = self.merge {
            match merge {
                serde_yaml::Value::Bool(true) => {
                    options.merge = Some(bibtex_tidy_rs::MergeStrategy::Combine);
                }
                serde_yaml::Value::Bool(false) => options.merge = None,
                serde_yaml::Value::String(value) => {
                    options.merge = match value.as_str() {
                        "first" => Some(bibtex_tidy_rs::MergeStrategy::First),
                        "last" => Some(bibtex_tidy_rs::MergeStrategy::Last),
                        "combine" => Some(bibtex_tidy_rs::MergeStrategy::Combine),
                        "overwrite" => Some(bibtex_tidy_rs::MergeStrategy::Overwrite),
                        _ => None,
                    };
                }
                _ => {}
            }
        }
        if let Some(months) = self.months {
            options.months = months;
        }
        if let Some(numeric) = self.numeric {
            options.numeric = numeric;
        }
        if let Some(omit) = self.omit {
            options.omit = omit;
        }
        if let Some(remove_braces) = self.remove_braces {
            match remove_braces {
                serde_yaml::Value::Bool(true) => options = options.with_remove_braces(),
                serde_yaml::Value::Bool(false) => options.remove_braces = None,
                serde_yaml::Value::Sequence(values) => {
                    options.remove_braces = Some(string_sequence(values));
                }
                _ => {}
            }
        }
        if let Some(remove_duplicate_fields) = self.remove_duplicate_fields {
            options.remove_duplicate_fields = remove_duplicate_fields;
        }
        if let Some(remove_empty_fields) = self.remove_empty_fields {
            options.remove_empty_fields = remove_empty_fields;
        }
        if let Some(space) = self.space {
            match space {
                serde_yaml::Value::Bool(true) => options.space = 2,
                serde_yaml::Value::Bool(false) => {}
                serde_yaml::Value::Number(value) => {
                    if let Some(value) = value.as_u64() {
                        options.space = value as usize;
                    }
                }
                _ => {}
            }
        }
        if let Some(sort_fields) = self.sort_fields {
            match sort_fields {
                serde_yaml::Value::Bool(true) => options = options.with_sort_fields(),
                serde_yaml::Value::Bool(false) => options.sort_fields = None,
                serde_yaml::Value::Sequence(values) => {
                    options.sort_fields = Some(
                        values
                            .into_iter()
                            .filter_map(|value| value.as_str().map(str::to_string))
                            .collect(),
                    );
                }
                _ => {}
            }
        }
        if let Some(sort) = self.sort {
            match sort {
                serde_yaml::Value::Bool(true) => options = options.with_sort(),
                serde_yaml::Value::Bool(false) => options.sort = None,
                serde_yaml::Value::Sequence(values) => {
                    options.sort = Some(
                        values
                            .into_iter()
                            .filter_map(|value| value.as_str().map(str::to_string))
                            .collect(),
                    );
                }
                _ => {}
            }
        }
        if let Some(strip_comments) = self.strip_comments {
            options.strip_comments = strip_comments;
        }
        if let Some(strip_enclosing_braces) = self.strip_enclosing_braces {
            options.strip_enclosing_braces = strip_enclosing_braces;
        }
        if let Some(tab) = self.tab {
            options.tab = tab;
        }
        if let Some(tidy_comments) = self.tidy_comments {
            options.tidy_comments = tidy_comments;
        }
        if let Some(trailing_commas) = self.trailing_commas {
            options.trailing_commas = trailing_commas;
        }
        if let Some(wrap) = self.wrap {
            options.wrap = Some(wrap);
        }
        options
    }
}

fn assert_expected(name: &str, spec: &SpecDocument, result: &bibtex_tidy_rs::TidyResult) {
    let expected = spec
        .expected
        .as_ref()
        .unwrap_or_else(|| panic!("{name} did not declare expected output"));
    assert_eq!(result.bibtex, *expected, "{name}");
}

fn string_sequence(values: Vec<serde_yaml::Value>) -> Vec<String> {
    values
        .into_iter()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect()
}

fn duplicate_rules(value: serde_yaml::Value) -> Option<Vec<bibtex_tidy_rs::DuplicateRule>> {
    match value {
        serde_yaml::Value::Bool(true) => Some(vec![
            bibtex_tidy_rs::DuplicateRule::Doi,
            bibtex_tidy_rs::DuplicateRule::Citation,
            bibtex_tidy_rs::DuplicateRule::Abstract,
            bibtex_tidy_rs::DuplicateRule::Key,
        ]),
        serde_yaml::Value::Bool(false) => None,
        serde_yaml::Value::Sequence(values) => Some(
            values
                .into_iter()
                .filter_map(|value| {
                    let value = value.as_str()?;
                    match value {
                        "doi" => Some(bibtex_tidy_rs::DuplicateRule::Doi),
                        "key" => Some(bibtex_tidy_rs::DuplicateRule::Key),
                        "abstract" => Some(bibtex_tidy_rs::DuplicateRule::Abstract),
                        "citation" => Some(bibtex_tidy_rs::DuplicateRule::Citation),
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

fn read_spec(name: &str) -> SpecDocument {
    read_specs(name).into_iter().next().unwrap_or_else(|| {
        panic!("spec file {name} did not contain a document");
    })
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
