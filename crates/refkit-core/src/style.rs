use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

use hayagriva::archive;
use hayagriva::citationberg::{IndependentStyle, Style as CslStyle};

use crate::quoted;
use crate::style_analysis::{
    can_fast_render_single_citations, citation_depends_on_subsequent_names, citation_only_style,
    full_history_citation_style,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleError {
    CachePoisoned,
    DependentStyle(String),
    InvalidXml(String),
    UnknownBundledStyle(String),
}

impl fmt::Display for StyleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CachePoisoned => f.write_str("style cache lock is poisoned"),
            Self::DependentStyle(id) => {
                write!(
                    f,
                    "style {} is dependent and needs parent resolution",
                    quoted(id)
                )
            }
            Self::InvalidXml(message) => write!(f, "invalid CSL XML: {message}"),
            Self::UnknownBundledStyle(name) => {
                write!(f, "unknown bundled style {}", quoted(name))
            }
        }
    }
}

impl std::error::Error for StyleError {}

#[derive(Debug, Clone)]
pub struct PreparedStyle {
    pub(crate) inner: Arc<IndependentStyle>,
    pub(crate) citation_style: Arc<IndependentStyle>,
    pub(crate) standalone_style: Arc<IndependentStyle>,
    pub(crate) fast_citation_enabled: bool,
    pub(crate) subsequent_name_rules: bool,
}

impl PreparedStyle {
    pub(crate) fn new(inner: IndependentStyle) -> Self {
        let citation_style = full_history_citation_style(&inner).map(Arc::new);
        let standalone_style = Arc::new(citation_only_style(&inner));
        let fast_citation_enabled = can_fast_render_single_citations(&inner);
        let subsequent_name_rules = citation_depends_on_subsequent_names(&inner);
        let inner = Arc::new(inner);
        let citation_style = citation_style.unwrap_or_else(|| Arc::clone(&inner));
        Self {
            inner,
            citation_style,
            standalone_style,
            fast_citation_enabled,
            subsequent_name_rules,
        }
    }

    pub fn title(&self) -> &str {
        &self.inner.info.title.value
    }
}

pub fn load_prepared_style(name: &str) -> Result<Arc<PreparedStyle>, StyleError> {
    static STYLES: OnceLock<Mutex<HashMap<String, Arc<PreparedStyle>>>> = OnceLock::new();

    let key = name.to_ascii_lowercase();
    let cache = STYLES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(style) = cache
        .lock()
        .map_err(|_| StyleError::CachePoisoned)?
        .get(&key)
        .cloned()
    {
        return Ok(style);
    }

    let archived = archive::ArchivedStyle::by_name(&key)
        .ok_or_else(|| StyleError::UnknownBundledStyle(name.to_string()))?;
    let style = prepare_csl_style(name.to_string(), archived.get())?;
    let style = Arc::new(style);
    cache
        .lock()
        .map_err(|_| StyleError::CachePoisoned)?
        .insert(key, Arc::clone(&style));
    Ok(style)
}

pub fn prepare_style_from_xml(xml: &str) -> Result<PreparedStyle, StyleError> {
    let style = CslStyle::from_xml(xml).map_err(|err| StyleError::InvalidXml(err.to_string()))?;
    prepare_csl_style("xml".to_string(), style)
}

fn prepare_csl_style(id: String, style: CslStyle) -> Result<PreparedStyle, StyleError> {
    match style {
        CslStyle::Independent(style) => Ok(PreparedStyle::new(style)),
        CslStyle::Dependent(_) => Err(StyleError::DependentStyle(id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_style_loads_case_insensitive_style_metadata() {
        let first = load_prepared_style("apa").unwrap();
        let second = load_prepared_style("APA").unwrap();

        assert!(!first.title().is_empty());
        assert_eq!(second.title(), first.title());
    }

    #[test]
    fn dependent_styles_are_reported_explicitly() {
        let err = prepare_csl_style(
            "child".to_string(),
            CslStyle::from_xml(
                r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0">
  <info>
    <title>Child</title>
    <id>https://example.com/child</id>
    <link rel="independent-parent" href="https://example.com/parent"/>
    <updated>2024-01-01T00:00:00+00:00</updated>
  </info>
</style>"#,
            )
            .unwrap(),
        )
        .unwrap_err();

        assert_eq!(err, StyleError::DependentStyle("child".to_string()));
    }

    #[test]
    fn invalid_xml_keeps_parser_message() {
        let err = prepare_style_from_xml("<style>").unwrap_err();

        assert!(matches!(err, StyleError::InvalidXml(_)));
    }
}
