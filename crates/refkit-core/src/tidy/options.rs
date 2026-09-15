/// Default width used to align field assignments.
pub const DEFAULT_ALIGN: usize = 14;
/// Default number of spaces per field indentation level.
pub const DEFAULT_SPACE: usize = 2;
/// Default line width selected by [`TidyOptions::with_wrap`].
pub const DEFAULT_WRAP: usize = 80;
/// Default key template requiring author and year components.
pub const DEFAULT_KEY_TEMPLATE: &str =
    "[auth:required:lower][year:required][veryshorttitle:lower][duplicateNumber]";

/// Field precedence selected by [`TidyOptions::with_sort_fields`].
pub const DEFAULT_FIELD_SORT: &[&str] = &[
    "title",
    "shorttitle",
    "author",
    "year",
    "month",
    "day",
    "journal",
    "booktitle",
    "location",
    "on",
    "publisher",
    "address",
    "series",
    "volume",
    "number",
    "pages",
    "doi",
    "isbn",
    "issn",
    "url",
    "urldate",
    "copyright",
    "category",
    "note",
    "metadata",
];

/// Entry ordering selected by [`TidyOptions::with_sort`].
pub const DEFAULT_SORT: &[&str] = &["key"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
/// Deterministic matching signature used by tidy and duplicate review.
pub enum DuplicateRule {
    /// DOI signature normalized by the shared duplicate matcher.
    Doi,
    /// Case-folded entry key signature.
    Key,
    /// Normalized leading abstract text signature.
    Abstract,
    /// Combined author, title, and number signature.
    Citation,
}

impl DuplicateRule {
    #[must_use]
    /// Return the host-facing duplicate rule identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Doi => "doi",
            Self::Key => "key",
            Self::Abstract => "abstract",
            Self::Citation => "citation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Tidy's automatic strategy for a matched duplicate group.
pub enum MergeStrategy {
    /// Keep the first occurrence's fields.
    First,
    /// Keep the last occurrence's fields.
    Last,
    /// Combine fields while retaining earlier conflicting values.
    Combine,
    /// Combine fields while replacing conflicts with later values.
    Overwrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Independent source transformations applied by the deterministic formatter.
#[expect(
    clippy::struct_excessive_bools,
    reason = "These independent formatter switches mirror the public Python/TypeScript options contract, not mutually exclusive states."
)]
pub struct TidyOptions {
    /// Source field names to omit.
    pub omit: Vec<String>,
    /// Convert eligible quoted values to braced values.
    pub curly: bool,
    /// Emit eligible numeric values without surrounding delimiters.
    pub numeric: bool,
    /// Normalize month values to month macros.
    pub months: bool,
    /// Field indentation width when tabs are disabled.
    pub space: usize,
    /// Use tabs for field indentation.
    pub tab: bool,
    /// Assignment alignment width, or no alignment when absent.
    pub align: Option<usize>,
    /// Separate entry blocks with blank lines.
    pub blank_lines: bool,
    /// Entry sort keys, or retain source order when absent.
    pub sort: Option<Vec<String>>,
    /// Duplicate signature rules, or no duplicate handling when absent.
    pub duplicates: Option<Vec<DuplicateRule>>,
    /// Automatic merge strategy for matched duplicate groups.
    pub merge: Option<MergeStrategy>,
    /// Remove redundant whole-value brace layers.
    pub strip_enclosing_braces: bool,
    /// Normalize fully capitalized word runs.
    pub drop_all_caps: bool,
    /// Escape characters requiring TeX treatment.
    pub escape: bool,
    /// Field sort precedence, or retain field order when absent.
    pub sort_fields: Option<Vec<String>>,
    /// Remove source comments.
    pub strip_comments: bool,
    /// Retain a comma after the final field assignment.
    pub trailing_commas: bool,
    /// Percent-encode URL characters according to formatter rules.
    pub encode_urls: bool,
    /// Normalize comment layout when retaining comments.
    pub tidy_comments: bool,
    /// Remove fields whose values are empty.
    pub remove_empty_fields: bool,
    /// Remove repeated assignments for the same field name.
    pub remove_duplicate_fields: bool,
    /// Key-generation template, or preserve existing keys when absent.
    pub generate_keys: Option<String>,
    /// Maximum author count before abbreviation.
    pub max_authors: Option<usize>,
    /// Normalize entry and field names to lowercase.
    pub lowercase: bool,
    /// Fields whose values receive an enclosing brace layer.
    pub enclosing_braces: Option<Vec<String>>,
    /// Fields whose brace protection is removed.
    pub remove_braces: Option<Vec<String>>,
    /// Requested wrapping width, or no wrapping when absent.
    pub wrap: Option<usize>,
}

impl Default for TidyOptions {
    fn default() -> Self {
        Self {
            omit: Vec::new(),
            curly: false,
            numeric: false,
            months: false,
            space: DEFAULT_SPACE,
            tab: false,
            align: Some(DEFAULT_ALIGN),
            blank_lines: false,
            sort: None,
            duplicates: None,
            merge: None,
            strip_enclosing_braces: false,
            drop_all_caps: false,
            escape: true,
            sort_fields: None,
            strip_comments: false,
            trailing_commas: false,
            encode_urls: false,
            tidy_comments: true,
            remove_empty_fields: false,
            remove_duplicate_fields: true,
            generate_keys: None,
            max_authors: None,
            lowercase: true,
            enclosing_braces: None,
            remove_braces: None,
            wrap: None,
        }
    }
}

impl TidyOptions {
    #[must_use]
    /// Enable sorting by the default entry key precedence.
    pub fn with_sort(mut self) -> Self {
        self.sort = Some(
            DEFAULT_SORT
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        );
        self
    }

    #[must_use]
    /// Enable the default bibliography field ordering.
    pub fn with_sort_fields(mut self) -> Self {
        self.sort_fields = Some(
            DEFAULT_FIELD_SORT
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        );
        self
    }

    #[must_use]
    /// Enable the default author/year/title key template.
    pub fn with_generate_keys(mut self) -> Self {
        self.generate_keys = Some(DEFAULT_KEY_TEMPLATE.to_string());
        self
    }

    #[must_use]
    /// Add an enclosing brace layer to title values.
    pub fn with_enclosing_braces(mut self) -> Self {
        self.enclosing_braces = Some(vec!["title".to_string()]);
        self
    }

    #[must_use]
    /// Remove title brace protection.
    pub fn with_remove_braces(mut self) -> Self {
        self.remove_braces = Some(vec!["title".to_string()]);
        self
    }

    #[must_use]
    /// Enable wrapping at the default line width.
    pub fn with_wrap(mut self) -> Self {
        self.wrap = Some(DEFAULT_WRAP);
        self
    }

    #[must_use]
    /// Disable field-assignment alignment.
    pub fn without_align(mut self) -> Self {
        self.align = None;
        self
    }
}
