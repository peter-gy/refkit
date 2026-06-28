mod duplicates;
mod keys;
mod latex;
mod options;
mod render;
mod unicode;

use std::fmt;

use refkit_core::{RawDocument, RawSyntaxBlock};

pub use options::{DuplicateRule, MergeStrategy, TidyOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TidyResult {
    pub bibtex: String,
    pub warnings: Vec<TidyWarning>,
    pub count: usize,
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
            Self::MissingKey { .. } => "MISSING_KEY",
            Self::DuplicateEntry { .. } => "DUPLICATE_ENTRY",
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
            Self::Template(message) | Self::Name(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for TidyError {}

pub struct BibtexTidy {
    options: TidyOptions,
}

impl BibtexTidy {
    pub fn new(options: TidyOptions) -> Self {
        Self { options }
    }

    pub fn tidy(&self, input: &str) -> Result<TidyResult, TidyError> {
        tidy(input, self.options.clone())
    }
}

pub fn tidy(input: &str, options: TidyOptions) -> Result<TidyResult, TidyError> {
    let input = normalize_newlines(input);
    let doc = RawDocument::parse(&input);
    let syntax = doc.syntax();

    if let Some((raw, error, byte)) = syntax.blocks.iter().find_map(first_failed_block) {
        return Err(syntax_error(&input, raw, error, byte));
    }

    let mut warnings = syntax
        .entries
        .iter()
        .filter(|entry| entry.key.trim().is_empty())
        .map(|entry| TidyWarning::MissingKey {
            message: format!("{} entry does not have a citation key.", entry.kind),
        })
        .collect::<Vec<_>>();
    let key_plan = keys::generated_keys(&syntax, &options).map_err(TidyError::Template)?;
    let duplicate_plan = duplicates::duplicate_plan(&syntax, &options);
    warnings.extend(duplicate_plan.warnings.iter().cloned());

    Ok(TidyResult {
        bibtex: render::render_document(&syntax, &options, &duplicate_plan, &key_plan),
        warnings,
        count: syntax.entries.len(),
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

fn normalize_newlines(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tidy_formats_entry_through_shared_raw_parser() {
        let result = tidy(
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
        let error = tidy(
            "@article{broken,\n  title = {No close}\n",
            TidyOptions::default(),
        )
        .unwrap_err();

        assert!(matches!(error, TidyError::Syntax { .. }));
    }

    #[test]
    fn tidy_warns_for_entries_without_keys() {
        let result = tidy(
            "@article{\n  title = {An entry with no key}\n}\n",
            TidyOptions::default(),
        )
        .unwrap();

        assert_eq!(result.count, 1);
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].code(), "MISSING_KEY");
    }

    #[test]
    fn tidy_matches_upstream_default_fixture_slice() {
        let result = tidy(
            concat!(
                "@ARTICLE {feinberg1983technique,\n",
                "    number={1},\n",
                "    title={A technique for radiolabeling DNA restriction endonuclease fragments to high specific activity},\n",
                "  author=\"Feinberg, Andrew P and Vogelstein, Bert\",\n",
                "    journal    = {Analytical biochemistry},\n",
                "    volume = 132,\n",
                "    pages={6-13},\n",
                "    year={1983},\n",
                "    month={aug},\n",
                "    publisher={Elsevier},}\n",
            ),
            TidyOptions::default(),
        )
        .unwrap();

        assert_eq!(
            result.bibtex,
            concat!(
                "@article{feinberg1983technique,\n",
                "  number        = {1},\n",
                "  title         = {A technique for radiolabeling DNA restriction endonuclease fragments to high specific activity},\n",
                "  author        = \"Feinberg, Andrew P and Vogelstein, Bert\",\n",
                "  journal       = {Analytical biochemistry},\n",
                "  volume        = 132,\n",
                "  pages         = {6--13},\n",
                "  year          = {1983},\n",
                "  month         = {aug},\n",
                "  publisher     = {Elsevier}\n",
                "}\n",
            )
        );
    }

    #[test]
    fn tidy_abbreviates_april_to_apr() {
        let options = TidyOptions {
            months: true,
            ..TidyOptions::default()
        };

        let result = tidy(
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

        let result = tidy(
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
        assert!(!result.bibtex.contains("jan # \" old\""));
    }

    #[test]
    fn duplicate_merge_chains_into_retained_entry() {
        let options = TidyOptions {
            merge: Some(MergeStrategy::Combine),
            ..TidyOptions::default()
        };

        let result = tidy(
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
        assert!(!result.bibtex.contains("@article{b,"));
        assert!(!result.bibtex.contains("@article{c,"));
    }

    #[test]
    fn generate_keys_keeps_upstream_first_middle_last_author_behavior() {
        let options = TidyOptions::default().with_generate_keys();

        let result = tidy(
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

        let result = tidy("@article{zero,\n  number = {0}\n}\n", options).unwrap();

        assert!(result.bibtex.contains("number        = {0}\n"));
    }

    #[test]
    fn descending_sort_keeps_missing_values_last() {
        let options = TidyOptions {
            sort: Some(vec!["-year".to_string()]),
            ..TidyOptions::default()
        };

        let result = tidy(
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
        let result = tidy(
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
        let result = tidy(
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
