mod duplicates;
mod keys;
mod latex;
mod options;
mod references;
mod render;
mod unicode;

use std::borrow::Cow;
use std::fmt;

use crate::raw::{RawDocument, RawEntryId, RawSyntaxBlock};

pub use options::{DuplicateRule, MergeStrategy, TidyOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TidyResult {
    pub bibtex: String,
    pub warnings: Vec<TidyWarning>,
    pub count: usize,
    pub renames: Vec<TidyRename>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TidyRename {
    pub entry_id: RawEntryId,
    pub old_key: String,
    pub new_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TidyWarning {
    MissingKey {
        message: String,
    },
    DuplicateEntry {
        rule: DuplicateRule,
        message: String,
    },
}

impl TidyWarning {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingKey { .. } => "missing_key",
            Self::DuplicateEntry { .. } => "duplicate_entry",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::MissingKey { message } | Self::DuplicateEntry { message, .. } => message,
        }
    }

    pub fn rule(&self) -> Option<DuplicateRule> {
        match self {
            Self::DuplicateEntry { rule, .. } => Some(*rule),
            Self::MissingKey { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TidyError {
    Syntax {
        line: usize,
        column: usize,
        byte: usize,
        character: Option<char>,
        message: String,
    },
    Template(String),
    Name(String),
    Reference(String),
}

impl fmt::Display for TidyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax {
                line,
                column,
                message,
                ..
            } => write!(f, "line {line}:{column}: {message}"),
            Self::Template(message) | Self::Name(message) | Self::Reference(message) => {
                f.write_str(message)
            }
        }
    }
}

impl std::error::Error for TidyError {}

pub fn tidy_bibtex(input: &str, options: TidyOptions) -> Result<TidyResult, TidyError> {
    crate::library::validate_source(input).map_err(|diagnostic| {
        syntax_error(
            input,
            input,
            &diagnostic.message,
            diagnostic.span.map_or(0, |span| span.start),
        )
    })?;
    let input = normalize_newlines(input);
    let doc = RawDocument::parse(&input);
    let mut syntax = doc.into_syntax();

    if let Some((raw, error, byte)) = syntax.blocks.iter().find_map(first_failed_block) {
        return Err(syntax_error(&input, raw, error, byte));
    }

    for entry in &syntax.entries {
        for field in &entry.fields {
            for atom in &field.value_atoms {
                crate::library::validate_literal(&atom.value, field.span.start).map_err(
                    |diagnostic| {
                        syntax_error(
                            &input,
                            &atom.value,
                            &diagnostic.message,
                            diagnostic.span.map_or(field.span.start, |span| span.start),
                        )
                    },
                )?;
            }
        }
    }

    let mut warnings = syntax
        .entries
        .iter()
        .filter(|entry| entry.key.trim().is_empty())
        .map(|entry| TidyWarning::MissingKey {
            message: format!("{} entry does not have a citation key.", entry.kind),
        })
        .collect::<Vec<_>>();
    let original_keys = if options.generate_keys.is_some() || options.merge.is_some() {
        syntax
            .entries
            .iter()
            .map(|entry| (entry.id, entry.key.clone()))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut duplicate_plan = duplicates::duplicate_plan(&syntax, &options);
    warnings.extend(duplicate_plan.warnings.iter().cloned());
    duplicate_plan.apply(&mut syntax);
    let retained = syntax
        .entries
        .iter()
        .filter(|entry| !duplicate_plan.should_skip(entry.id));
    let key_plan = keys::generated_keys(retained, &options).map_err(TidyError::Template)?;
    for entry in &mut syntax.entries {
        if let Some(key) = key_plan.get(&entry.id) {
            entry.key.clone_from(key);
        }
    }
    let renames = original_keys
        .iter()
        .filter_map(|(id, old_key)| {
            let new_key = &syntax.entries[duplicate_plan.retained_id(*id).index()].key;
            (old_key != new_key).then(|| TidyRename {
                entry_id: *id,
                old_key: old_key.clone(),
                new_key: new_key.clone(),
            })
        })
        .collect::<Vec<_>>();
    if !renames.is_empty() {
        references::rewrite(
            &mut syntax,
            &original_keys,
            &duplicate_plan,
            &input,
            &options,
        )?;
    }

    Ok(TidyResult {
        bibtex: render::render_document(&syntax, &options, &duplicate_plan),
        warnings,
        count: syntax.entries.len(),
        renames,
    })
}

fn first_failed_block(block: &RawSyntaxBlock) -> Option<(&str, &str, usize)> {
    match block {
        RawSyntaxBlock::Failed { raw, error, span } => {
            Some((raw.as_str(), error.as_str(), span.start))
        }
        _ => None,
    }
}

fn syntax_error(input: &str, raw: &str, message: &str, byte: usize) -> TidyError {
    let (line, column) = line_column(input, byte);
    TidyError::Syntax {
        line,
        column,
        byte,
        character: raw.chars().next(),
        message: message.to_string(),
    }
}

fn line_column(input: &str, byte: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    for (idx, ch) in input.char_indices() {
        if idx >= byte {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

fn normalize_newlines(input: &str) -> Cow<'_, str> {
    if input.contains('\r') {
        Cow::Owned(input.replace("\r\n", "\n").replace('\r', "\n"))
    } else {
        Cow::Borrowed(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tidy_formats_entry_through_shared_raw_parser() {
        let result = tidy_bibtex(
            concat!(
                "@ARTICLE {feinberg1983technique,\n",
                "    number={1},\n",
                "  pages={6-13},\n",
                "  year={1983},}\n",
            ),
            TidyOptions::default(),
        )
        .unwrap();

        assert_eq!(
            result.bibtex,
            concat!(
                "@article{feinberg1983technique,\n",
                "  number        = {1},\n",
                "  pages         = {6--13},\n",
                "  year          = {1983}\n",
                "}\n",
            )
        );
        assert_eq!(result.count, 1);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn tidy_reports_shared_parser_failures_as_syntax_errors() {
        let error = tidy_bibtex(
            "@article{broken,\n  title = {No close}\n",
            TidyOptions::default(),
        )
        .unwrap_err();

        assert!(matches!(error, TidyError::Syntax { .. }));
    }

    #[test]
    fn tidy_warns_for_entries_without_keys() {
        let result = tidy_bibtex(
            "@article{\n  title = {An entry with no key}\n}\n",
            TidyOptions::default(),
        )
        .unwrap();

        assert_eq!(result.count, 1);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].code(), "missing_key");
    }

    #[test]
    fn tidy_abbreviates_april_to_apr() {
        let options = TidyOptions {
            months: true,
            ..TidyOptions::default()
        };

        let result = tidy_bibtex(
            "@article{month,\n  title = {Month},\n  month = {april}\n}\n",
            options,
        )
        .unwrap();

        assert!(result.bibtex.contains("month         = apr\n"));
    }

    #[test]
    fn overwrite_merge_updates_expression_values() {
        let options = TidyOptions {
            merge: Some(MergeStrategy::Overwrite),
            ..TidyOptions::default()
        };

        let result = tidy_bibtex(
            concat!(
                "@article{a,\n",
                "  title = {Old},\n",
                "  doi = {10.1000/example},\n",
                "  note = jan # \" old\"\n",
                "}\n",
                "@article{b,\n",
                "  title = {New},\n",
                "  doi = {10.1000/example},\n",
                "  note = feb # \" new\"\n",
                "}\n",
            ),
            options,
        )
        .unwrap();

        assert_eq!(result.count, 2);
        assert_eq!(result.warnings.len(), 1);
        assert!(result.bibtex.contains("title         = {New},\n"));
        assert!(result.bibtex.contains("note          = feb # \" new\"\n"));
    }

    #[test]
    fn duplicate_merge_chains_into_retained_entry() {
        let options = TidyOptions {
            merge: Some(MergeStrategy::Combine),
            ..TidyOptions::default()
        };

        let result = tidy_bibtex(
            concat!(
                "@article{a,\n",
                "  title = {Anchor},\n",
                "  author = {Alpha, A},\n",
                "  number = {1},\n",
                "  doi = {10.1000/same},\n",
                "  anchor = {A}\n",
                "}\n",
                "@article{b,\n",
                "  title = {Shared},\n",
                "  author = {Beta, B},\n",
                "  number = {2},\n",
                "  doi = {10.1000/same},\n",
                "  middle = {B}\n",
                "}\n",
                "@article{c,\n",
                "  title = {Shared},\n",
                "  author = {Beta, B},\n",
                "  number = {2},\n",
                "  doi = {10.1000/other},\n",
                "  tail = {C}\n",
                "}\n",
            ),
            options,
        )
        .unwrap();

        assert!(result.bibtex.contains("middle        = {B},\n"));
        assert!(result.bibtex.contains("tail          = {C}\n"));
        assert_eq!(RawDocument::parse(&result.bibtex).entry_keys(), ["a"]);
    }

    #[test]
    fn generate_keys_keeps_upstream_first_middle_last_author_behavior() {
        let options = TidyOptions::default().with_generate_keys();

        let result = tidy_bibtex(
            concat!(
                "@article{x,\n",
                "  author = {John Q. Public},\n",
                "  title = {Example Title},\n",
                "  year = {2024}\n",
                "}\n",
            ),
            options,
        )
        .unwrap();

        assert!(result.bibtex.starts_with("@article{qpublic2024example,"));
    }

    #[test]
    fn numeric_zero_remains_braced_like_upstream() {
        let options = TidyOptions {
            numeric: true,
            ..TidyOptions::default()
        };

        let result = tidy_bibtex("@article{zero,\n  number = {0}\n}\n", options).unwrap();

        assert!(result.bibtex.contains("number        = {0}\n"));
    }

    #[test]
    fn descending_sort_keeps_missing_values_last() {
        let options = TidyOptions {
            sort: Some(vec!["-year".to_string()]),
            ..TidyOptions::default()
        };

        let result = tidy_bibtex(
            concat!(
                "@article{missing,\n",
                "  title = {Missing}\n",
                "}\n",
                "@article{newer,\n",
                "  title = {Newer},\n",
                "  year = {2024}\n",
                "}\n",
                "@article{older,\n",
                "  title = {Older},\n",
                "  year = {2020}\n",
                "}\n",
            ),
            options,
        )
        .unwrap();

        let newer = result.bibtex.find("@article{newer,").unwrap();
        let older = result.bibtex.find("@article{older,").unwrap();
        let missing = result.bibtex.find("@article{missing,").unwrap();
        assert!(newer < older);
        assert!(older < missing);
    }

    #[test]
    fn tidy_recovers_parseable_blocks_inside_percent_comment_lines_like_upstream() {
        let result = tidy_bibtex(
            "% @article{commented,title={Hidden}}\n@article{live,title={Shown}}\n",
            TidyOptions::default(),
        )
        .unwrap();

        assert_eq!(
            result.bibtex,
            concat!(
                "%\n",
                "@article{commented,\n",
                "  title         = {Hidden}\n",
                "}\n",
                "@article{live,\n",
                "  title         = {Shown}\n",
                "}\n",
            )
        );
    }

    #[test]
    fn tidy_keeps_incomplete_at_blocks_inside_percent_comments_inert() {
        let result = tidy_bibtex(
            "% see @article{not a record\n@article{real,title={Shown}}\n",
            TidyOptions::default(),
        )
        .unwrap();

        assert_eq!(
            result.bibtex,
            concat!(
                "% see @article{not a record\n",
                "@article{real,\n",
                "  title         = {Shown}\n",
                "}\n",
            )
        );
    }
}
