mod broadcast;
mod dtypes;
mod entries;
mod parse;
mod render;
mod tidy;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct ParseKwargs {
    pub(super) strict: bool,
}

#[derive(Debug, Deserialize)]
pub(super) struct EntriesKwargs {
    pub(super) strict: bool,
    pub(super) fields: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct RenderKwargs {
    pub(super) style: String,
    pub(super) locale: String,
    pub(super) strict: bool,
    pub(super) all: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TidyKwargs {
    #[serde(rename = "_defaults")]
    pub(super) _defaults: Option<bool>,
    pub(super) omit: Option<Vec<String>>,
    pub(super) curly: Option<bool>,
    pub(super) numeric: Option<bool>,
    pub(super) months: Option<bool>,
    pub(super) space: Option<usize>,
    pub(super) tab: Option<bool>,
    pub(super) align: Option<DefaultableUsize>,
    pub(super) blank_lines: Option<bool>,
    pub(super) sort: Option<DefaultableStringList>,
    pub(super) duplicates: Option<Vec<String>>,
    pub(super) merge: Option<String>,
    pub(super) strip_enclosing_braces: Option<bool>,
    pub(super) drop_all_caps: Option<bool>,
    pub(super) escape: Option<bool>,
    pub(super) sort_fields: Option<DefaultableStringList>,
    pub(super) strip_comments: Option<bool>,
    pub(super) trailing_commas: Option<bool>,
    pub(super) encode_urls: Option<bool>,
    pub(super) tidy_comments: Option<bool>,
    pub(super) remove_empty_fields: Option<bool>,
    pub(super) remove_duplicate_fields: Option<bool>,
    pub(super) generate_keys: Option<DefaultableString>,
    pub(super) max_authors: Option<usize>,
    pub(super) lowercase: Option<bool>,
    pub(super) enclosing_braces: Option<DefaultableStringList>,
    pub(super) remove_braces: Option<DefaultableStringList>,
    pub(super) wrap: Option<DefaultableUsize>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum DefaultableUsize {
    Enabled(bool),
    Value(usize),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum DefaultableString {
    Enabled(bool),
    Value(String),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(super) enum DefaultableStringList {
    Enabled(bool),
    Values(Vec<String>),
}
