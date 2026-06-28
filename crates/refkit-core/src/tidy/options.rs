pub const DEFAULT_ALIGN: usize = 14;
pub const DEFAULT_SPACE: usize = 2;
pub const DEFAULT_WRAP: usize = 80;
pub const DEFAULT_KEY_TEMPLATE: &str =
    "[auth:required:lower][year:required][veryshorttitle:lower][duplicateNumber]";

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

pub const DEFAULT_SORT: &[&str] = &["key"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DuplicateRule {
    Doi,
    Key,
    Abstract,
    Citation,
}

impl DuplicateRule {
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
pub enum MergeStrategy {
    First,
    Last,
    Combine,
    Overwrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TidyOptions {
    pub omit: Vec<String>,
    pub curly: bool,
    pub numeric: bool,
    pub months: bool,
    pub space: usize,
    pub tab: bool,
    pub align: Option<usize>,
    pub blank_lines: bool,
    pub sort: Option<Vec<String>>,
    pub duplicates: Option<Vec<DuplicateRule>>,
    pub merge: Option<MergeStrategy>,
    pub strip_enclosing_braces: bool,
    pub drop_all_caps: bool,
    pub escape: bool,
    pub sort_fields: Option<Vec<String>>,
    pub strip_comments: bool,
    pub trailing_commas: bool,
    pub encode_urls: bool,
    pub tidy_comments: bool,
    pub remove_empty_fields: bool,
    pub remove_duplicate_fields: bool,
    pub generate_keys: Option<String>,
    pub max_authors: Option<usize>,
    pub lowercase: bool,
    pub enclosing_braces: Option<Vec<String>>,
    pub remove_braces: Option<Vec<String>>,
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
    pub fn with_sort(mut self) -> Self {
        self.sort = Some(
            DEFAULT_SORT
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        );
        self
    }

    pub fn with_sort_fields(mut self) -> Self {
        self.sort_fields = Some(
            DEFAULT_FIELD_SORT
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        );
        self
    }

    pub fn with_generate_keys(mut self) -> Self {
        self.generate_keys = Some(DEFAULT_KEY_TEMPLATE.to_string());
        self
    }

    pub fn with_enclosing_braces(mut self) -> Self {
        self.enclosing_braces = Some(vec!["title".to_string()]);
        self
    }

    pub fn with_remove_braces(mut self) -> Self {
        self.remove_braces = Some(vec!["title".to_string()]);
        self
    }

    pub fn with_wrap(mut self) -> Self {
        self.wrap = Some(DEFAULT_WRAP);
        self
    }

    pub fn without_align(mut self) -> Self {
        self.align = None;
        self
    }
}
