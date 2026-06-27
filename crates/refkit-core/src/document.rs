use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use hayagriva::citationberg::taxonomy::Locator as CslLocator;
use hayagriva::citationberg::{IndependentStyle, LocaleCode};
use hayagriva::{
    BibliographyDriver, BibliographyRequest, CitationItem, CitationRequest, Entry as HayEntry,
    LocatorPayload, Rendered as HayRendered, SpecificLocator,
};

use crate::render::bundled_locales;
use crate::render_tree::{rendered_record_from_bibliography, rendered_record_from_citation};
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
}

#[derive(Debug)]
pub struct RenderedDocument {
    pub citations: Vec<RenderedRecord>,
    pub bibliography: RenderedRecord,
}

impl Document {
    pub fn new(
        library: Arc<CoreLibrary>,
        style: Arc<PreparedStyle>,
        locale: Option<String>,
    ) -> Self {
        Self {
            library,
            style,
            locale,
        }
    }

    pub fn entry_count(&self) -> usize {
        self.library.len()
    }

    pub fn render(&self, citations: Vec<Vec<Cite>>) -> Result<RenderedDocument, DocumentError> {
        let rendered = self.render_with_citations(&citations, false, self.style.inner.as_ref())?;
        let citations = rendered
            .citations
            .iter()
            .map(rendered_record_from_citation)
            .collect::<Result<Vec<_>, _>>()
            .map_err(DocumentError::Render)?;
        let bibliography = rendered_record_from_bibliography(rendered.bibliography)
            .map_err(DocumentError::Render)?;
        Ok(RenderedDocument {
            citations,
            bibliography,
        })
    }

    pub fn cited_bibliography(
        &self,
        citations: Vec<Vec<Cite>>,
    ) -> Result<RenderedRecord, DocumentError> {
        let rendered = self.render_with_citations(&citations, false, self.style.inner.as_ref())?;
        rendered_record_from_bibliography(rendered.bibliography).map_err(DocumentError::Render)
    }

    pub fn full_bibliography(&self) -> Result<RenderedRecord, DocumentError> {
        let rendered = self.render_with_citations(&[], true, self.style.inner.as_ref())?;
        rendered_record_from_bibliography(rendered.bibliography).map_err(DocumentError::Render)
    }

    fn render_with_citations(
        &self,
        citations: &[Vec<Cite>],
        all: bool,
        style: &IndependentStyle,
    ) -> Result<HayRendered, DocumentError> {
        let locales = bundled_locales();
        let locale = self.locale.as_ref().map(|code| LocaleCode(code.clone()));
        let mut driver = BibliographyDriver::new();

        for group in citations {
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
    fn missing_reference_fails_whole_document_render() {
        let document = test_document(
            "apa",
            "@article{valid, author = {Doe, Jane}, title = {Valid}, year = {2024}}",
        );

        let err = document
            .render(vec![vec![test_cite("valid")], vec![test_cite("missing")]])
            .unwrap_err();

        assert_eq!(err, DocumentError::MissingReference("missing".to_string()));
    }

    #[test]
    fn invalid_locator_label_is_structured_error() {
        let document = test_document(
            "apa",
            "@article{valid, author = {Doe, Jane}, title = {Valid}, year = {2024}}",
        );

        let err = document
            .render(vec![vec![Cite::new(
                "valid".to_string(),
                Some("12".to_string()),
                Some("nonsense".to_string()),
            )]])
            .unwrap_err();

        assert_eq!(
            err,
            DocumentError::UnknownLocatorLabel("nonsense".to_string())
        );
    }

    #[test]
    fn bibliography_scope_is_explicit() {
        let document = test_document(
            "ieee",
            concat!(
                "@article{a, author = {Doe, Jane}, title = {A}, year = {2024}}\n",
                "@article{b, author = {Roe, Jane}, title = {B}, year = {2025}}\n",
            ),
        );

        let rendered = document.render(vec![vec![test_cite("b")]]).unwrap();
        let cited = document
            .cited_bibliography(vec![vec![test_cite("b")]])
            .unwrap();
        let full = document.full_bibliography().unwrap();

        assert!(rendered.citations[0].text.contains("[1]"));
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
