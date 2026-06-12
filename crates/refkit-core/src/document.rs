use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use hayagriva::citationberg::taxonomy::Locator as CslLocator;
use hayagriva::citationberg::{IndependentStyle, LocaleCode};
use hayagriva::{
    BibliographyDriver, BibliographyRequest, BufWriteFormat, CitationItem, CitationRequest,
    Entry as HayEntry, LocatorPayload, Rendered as HayRendered, SpecificLocator,
    standalone_citation,
};

use crate::render::{bundled_locales, elem_children_to_html, elem_children_to_string};
use crate::render_tree::{
    rendered_record_from_bibliography, rendered_record_from_citation,
    rendered_record_from_citation_parts,
};
use crate::{CoreLibrary, PreparedStyle, RenderedRecord};

#[derive(Debug, Clone)]
pub struct Cite {
    pub key: String,
    pub locator: Option<String>,
    pub label: Option<String>,
}

impl Cite {
    pub fn new(key: String, locator: Option<String>, label: Option<String>) -> Self {
        Self {
            key,
            locator,
            label,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentError {
    MissingReference(String),
    UnknownLocatorLabel(String),
    Render(String),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingReference(key) => write!(f, "missing reference {key}"),
            Self::UnknownLocatorLabel(label) => write!(f, "unknown locator label {label}"),
            Self::Render(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for DocumentError {}

#[derive(Clone)]
pub struct Document {
    library: Arc<CoreLibrary>,
    style: Arc<PreparedStyle>,
    locale: Option<String>,
    citations: Vec<Vec<Cite>>,
    fast_cite: FastCitationState,
}

#[derive(Clone)]
struct FastCitationState {
    enabled: bool,
    key_by_text: HashMap<String, String>,
    seen_keys: HashSet<String>,
    subsequent_name_rules: bool,
}

impl FastCitationState {
    fn new(fast_citation_enabled: bool, subsequent_name_rules: bool) -> Self {
        Self {
            enabled: fast_citation_enabled,
            key_by_text: HashMap::new(),
            seen_keys: HashSet::new(),
            subsequent_name_rules,
        }
    }
}

impl Document {
    pub fn new(
        library: Arc<CoreLibrary>,
        style: Arc<PreparedStyle>,
        locale: Option<String>,
    ) -> Self {
        let fast_cite =
            FastCitationState::new(style.fast_citation_enabled, style.subsequent_name_rules);
        Self {
            library,
            style,
            locale,
            citations: Vec::new(),
            fast_cite,
        }
    }

    pub fn entry_count(&self) -> usize {
        self.library.len()
    }

    pub fn citation_count(&self) -> usize {
        self.citations.len()
    }

    pub fn cite_group(&mut self, group: Vec<Cite>) -> Result<RenderedRecord, DocumentError> {
        let fast_cite = self.fast_cite.clone();
        self.citations.push(group);
        match self.render_appended_citation() {
            Ok(rendered) => Ok(rendered),
            Err(err) => {
                self.citations.pop();
                self.fast_cite = fast_cite;
                Err(err)
            }
        }
    }

    pub fn bibliography(&self, all: bool) -> Result<RenderedRecord, DocumentError> {
        let rendered = self.render_all(all)?;
        rendered_record_from_bibliography(rendered.bibliography).map_err(DocumentError::Render)
    }

    fn render_appended_citation(&mut self) -> Result<RenderedRecord, DocumentError> {
        if let Some(rendered) = self.try_render_fast_citation()? {
            return Ok(rendered);
        }
        self.render_latest_citation()
    }

    fn try_render_fast_citation(&mut self) -> Result<Option<RenderedRecord>, DocumentError> {
        if !self.fast_cite.enabled {
            return Ok(None);
        }

        let Some(group) = self.citations.last() else {
            return Ok(None);
        };
        let [cite] = group.as_slice() else {
            self.fast_cite.enabled = false;
            return Ok(None);
        };
        if cite.locator.is_some() {
            self.fast_cite.enabled = false;
            return Ok(None);
        }

        let entry = self
            .library
            .inner()
            .get(&cite.key)
            .ok_or_else(|| DocumentError::MissingReference(cite.key.clone()))?;
        if self.fast_cite.subsequent_name_rules && self.fast_cite.seen_keys.contains(&cite.key) {
            return Ok(None);
        }

        let locale = self.locale.as_ref().map(|code| LocaleCode(code.clone()));
        let children = standalone_citation(CitationRequest::new(
            vec![citation_item(entry, cite)?],
            self.style.standalone_style.as_ref(),
            locale,
            bundled_locales(),
            None,
        ));
        let text = elem_children_to_string(&children, BufWriteFormat::Plain)
            .map_err(DocumentError::Render)?;

        match self.fast_cite.key_by_text.get(&text) {
            Some(existing_key) if existing_key != &cite.key => {
                self.fast_cite.enabled = false;
                Ok(None)
            }
            _ => {
                self.fast_cite
                    .key_by_text
                    .insert(text.clone(), cite.key.clone());
                self.fast_cite.seen_keys.insert(cite.key.clone());
                let html = elem_children_to_html(&children).map_err(DocumentError::Render)?;
                Ok(Some(rendered_record_from_citation_parts(
                    text, html, children,
                )))
            }
        }
    }

    fn render_latest_citation(&self) -> Result<RenderedRecord, DocumentError> {
        let rendered = self.render_with_style(false, self.style.citation_style.as_ref())?;
        let Some(citation) = rendered.citations.last() else {
            return Err(DocumentError::Render(
                "citation renderer returned no citations".to_string(),
            ));
        };
        rendered_record_from_citation(citation).map_err(DocumentError::Render)
    }

    fn render_all(&self, all: bool) -> Result<HayRendered, DocumentError> {
        self.render_with_style(all, self.style.inner.as_ref())
    }

    fn render_with_style(
        &self,
        all: bool,
        style: &IndependentStyle,
    ) -> Result<HayRendered, DocumentError> {
        let locales = bundled_locales();
        let locale = self.locale.as_ref().map(|code| LocaleCode(code.clone()));
        let mut driver = BibliographyDriver::new();

        for group in &self.citations {
            let mut items = Vec::with_capacity(group.len());
            for cite in group {
                let entry = self
                    .library
                    .inner()
                    .get(&cite.key)
                    .ok_or_else(|| DocumentError::MissingReference(cite.key.clone()))?;
                items.push(citation_item(entry, cite)?);
            }

            driver.citation(CitationRequest::new(
                items,
                style,
                locale.clone(),
                locales,
                None,
            ));
        }

        if all {
            for entry in self.library.inner().iter() {
                driver.citation(CitationRequest::new(
                    vec![CitationItem::with_entry(entry)],
                    style,
                    locale.clone(),
                    locales,
                    None,
                ));
            }
        }

        Ok(driver.finish(BibliographyRequest::new(style, locale, locales)))
    }
}

fn citation_item<'a>(
    entry: &'a HayEntry,
    cite: &'a Cite,
) -> Result<CitationItem<'a, HayEntry>, DocumentError> {
    let locator = match cite.locator.as_deref() {
        Some(value) => {
            let label = cite.label.as_deref().unwrap_or("page");
            let locator = CslLocator::from_str(label)
                .map_err(|_| DocumentError::UnknownLocatorLabel(label.to_string()))?;
            Some(SpecificLocator(locator, LocatorPayload::Str(value)))
        }
        None => None,
    };
    Ok(CitationItem::with_locator(entry, locator))
}

#[cfg(test)]
mod tests {
    use crate::{CoreLibrary, load_prepared_style};

    use super::*;

    #[test]
    fn failed_group_citation_rolls_back_state() {
        let mut document = test_document(
            "apa",
            "@article{valid, author = {Doe, Jane}, title = {Valid}, year = {2024}}",
        );

        assert!(
            document
                .cite_group(vec![test_cite("valid"), test_cite("missing")])
                .is_err()
        );

        assert_eq!(document.citation_count(), 0);

        let rendered = document.cite_group(vec![test_cite("valid")]).unwrap();
        assert!(rendered.text.contains("Doe"));
    }

    #[test]
    fn invalid_locator_label_is_structured_error() {
        let mut document = test_document(
            "apa",
            "@article{valid, author = {Doe, Jane}, title = {Valid}, year = {2024}}",
        );

        let err = document
            .cite_group(vec![Cite::new(
                "valid".to_string(),
                Some("12".to_string()),
                Some("nonsense".to_string()),
            )])
            .unwrap_err();

        assert_eq!(
            err,
            DocumentError::UnknownLocatorLabel("nonsense".to_string())
        );
        assert_eq!(document.citation_count(), 0);
    }

    #[test]
    fn bibliography_uses_citation_history() {
        let mut document = test_document(
            "ieee",
            concat!(
                "@article{a, author = {Doe, Jane}, title = {A}, year = {2024}}\n",
                "@article{b, author = {Roe, Jane}, title = {B}, year = {2025}}\n",
            ),
        );

        document.cite_group(vec![test_cite("b")]).unwrap();
        let cited = document.bibliography(false).unwrap();
        let full = document.bibliography(true).unwrap();

        assert!(cited.text.contains("Roe"));
        assert!(!cited.text.contains("Doe"));
        assert!(full.text.contains("Roe"));
        assert!(full.text.contains("Doe"));
    }

    fn test_document(style: &str, source: &str) -> Document {
        let library = Arc::new(CoreLibrary::parse_source(source, "bibtex", false, false).unwrap());
        let style = load_prepared_style(style).unwrap();
        Document::new(library, style, Some("en-US".to_string()))
    }

    fn test_cite(key: &str) -> Cite {
        Cite::new(key.to_string(), None, None)
    }
}
