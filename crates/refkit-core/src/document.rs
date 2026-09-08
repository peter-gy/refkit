use std::fmt;
use std::sync::Arc;

use crate::render::{full_bibliography_requests, process_citations};
use crate::render_tree::{rendered_record_from_bibliography, rendered_record_from_citation};
use crate::{Library, PreparedStyle, RenderedRecord};

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

#[derive(Debug, Clone)]
pub struct CitationRequest {
    pub items: Vec<Cite>,
    pub note_number: Option<usize>,
}

impl CitationRequest {
    pub fn new(items: Vec<Cite>, note_number: Option<usize>) -> Self {
        Self { items, note_number }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentError {
    MissingReference(String),
    UnknownLocatorLabel(String),
    EmptyCitation,
    InvalidNoteNumber,
    Render(String),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingReference(key) => write!(f, "missing reference {}", crate::quoted(key)),
            Self::UnknownLocatorLabel(label) => {
                write!(f, "unknown locator label {}", crate::quoted(label))
            }
            Self::EmptyCitation => f.write_str("citation requires at least one item"),
            Self::InvalidNoteNumber => f.write_str("note number must be between 1 and 4294967295"),
            Self::Render(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for DocumentError {}

#[derive(Clone)]
pub struct Document {
    library: Arc<Library>,
    style: Arc<PreparedStyle>,
    locale: Option<String>,
}

#[derive(Debug)]
pub struct RenderedDocument {
    pub citations: Vec<RenderedRecord>,
    pub bibliography: RenderedRecord,
}

impl Document {
    pub fn new(library: Arc<Library>, style: Arc<PreparedStyle>, locale: Option<String>) -> Self {
        Self {
            library,
            style,
            locale,
        }
    }

    pub fn entry_count(&self) -> usize {
        self.library.len()
    }

    pub fn render(
        &self,
        requests: Vec<CitationRequest>,
    ) -> Result<RenderedDocument, DocumentError> {
        let rendered = process_citations(
            &self.library,
            &self.style,
            self.locale.as_deref(),
            &requests,
        )?;
        let citations = rendered
            .citations
            .into_iter()
            .zip(requests)
            .map(|(citation, request)| {
                rendered_record_from_citation(
                    citation,
                    request.items.into_iter().map(|item| item.key).collect(),
                )
            })
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
        requests: Vec<CitationRequest>,
    ) -> Result<RenderedRecord, DocumentError> {
        let rendered = process_citations(
            &self.library,
            &self.style,
            self.locale.as_deref(),
            &requests,
        )?;
        rendered_record_from_bibliography(rendered.bibliography).map_err(DocumentError::Render)
    }

    pub fn full_bibliography(&self) -> Result<RenderedRecord, DocumentError> {
        self.cited_bibliography(full_bibliography_requests(&self.library))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Library, RecoveryPolicy, load_prepared_style};

    use super::*;

    #[test]
    fn missing_reference_fails_whole_document_render() {
        let document = test_document(
            "apa",
            "@article{valid, author = {Doe, Jane}, title = {Valid}, year = {2024}}",
        );

        let err = document
            .render(vec![test_request("valid"), test_request("missing")])
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
            .render(vec![CitationRequest {
                items: vec![Cite::new(
                    "valid".to_string(),
                    Some("12".to_string()),
                    Some("nonsense".to_string()),
                )],
                note_number: None,
            }])
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

        let rendered = document.render(vec![test_request("b")]).unwrap();
        let cited = document
            .cited_bibliography(vec![test_request("b")])
            .unwrap();
        let full = document.full_bibliography().unwrap();

        assert!(rendered.citations[0].text.contains("[1]"));
        assert!(cited.text.contains("Roe"));
        assert!(!cited.text.contains("Doe"));
        assert!(full.text.contains("Roe"));
        assert!(full.text.contains("Doe"));
    }

    fn test_document(style: &str, source: &str) -> Document {
        let library = Arc::new(Library::parse_biblatex(source, RecoveryPolicy::Report).unwrap());
        let style = load_prepared_style(style).unwrap();
        Document::new(library, style, Some("en-US".to_string()))
    }

    fn test_request(key: &str) -> CitationRequest {
        CitationRequest::new(vec![Cite::new(key.to_string(), None, None)], None)
    }
}
