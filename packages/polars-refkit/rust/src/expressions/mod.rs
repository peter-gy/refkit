mod broadcast;
mod dtypes;
mod entries;
mod parse;
mod render;

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
