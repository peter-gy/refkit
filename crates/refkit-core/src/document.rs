use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use crate::render::{full_bibliography_requests, process_citations};
use crate::render_tree::{rendered_record_from_bibliography, rendered_record_from_citation};
use crate::{Library, PreparedStyle, RenderedRecord};

#[derive(Debug, Clone)]
/// One library reference within a citation request.
pub struct Cite {
    /// Key of an entry in the prepared library.
    pub key: String,
    /// Locator text such as a page or chapter range.
    pub locator: Option<String>,
    /// CSL locator label, defaulting to page when a locator is supplied.
    pub label: Option<String>,
    /// Requested style-supported citation form.
    pub purpose: CitePurpose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// Style-dependent treatment of one citation item.
pub enum CitePurpose {
    #[default]
    /// Ordinary citation rendering according to the style.
    Normal,
    /// Render the citation's author component.
    Author,
    /// Render the citation's year component.
    Year,
    /// Render the full reference form.
    Full,
    /// Render a narrative citation for use in prose.
    Prose,
}

impl CitePurpose {
    #[must_use]
    /// Return the purpose identifier accepted by host adapters.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Author => "author",
            Self::Year => "year",
            Self::Full => "full",
            Self::Prose => "prose",
        }
    }
}

impl FromStr for CitePurpose {
    type Err = DocumentError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "normal" => Ok(Self::Normal),
            "author" => Ok(Self::Author),
            "year" => Ok(Self::Year),
            "full" => Ok(Self::Full),
            "prose" => Ok(Self::Prose),
            _ => Err(DocumentError::UnknownCitationPurpose(value.to_string())),
        }
    }
}

impl Cite {
    #[must_use]
    /// Construct a request item. Reference and locator validation occurs at rendering.
    pub fn new(
        key: String,
        locator: Option<String>,
        label: Option<String>,
        purpose: CitePurpose,
    ) -> Self {
        Self {
            key,
            locator,
            label,
            purpose,
        }
    }
}

#[derive(Debug, Clone)]
/// Ordered citation items with optional note context.
pub struct CitationRequest {
    /// Citation items in requested order. Rendering rejects an empty list.
    pub items: Vec<Cite>,
    /// One-based note number, bounded to the portable unsigned 32-bit range.
    pub note_number: Option<usize>,
}

impl CitationRequest {
    #[must_use]
    /// Construct a citation request for validation during rendering.
    pub fn new(items: Vec<Cite>, note_number: Option<usize>) -> Self {
        Self { items, note_number }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Invalid citation input or a failure to materialize rendered output.
pub enum DocumentError {
    /// A requested key is absent from the prepared library.
    MissingReference(String),
    /// A locator label is outside the supported CSL vocabulary.
    UnknownLocatorLabel(String),
    /// A citation purpose is outside the supported vocabulary.
    UnknownCitationPurpose(String),
    /// A citation request contains no items.
    EmptyCitation,
    /// A note number is zero or exceeds the portable range.
    InvalidNoteNumber,
    /// The renderer could not produce the requested text, HTML, or tree.
    Render(String),
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingReference(key) => write!(f, "missing reference {}", crate::quoted(key)),
            Self::UnknownLocatorLabel(label) => {
                write!(f, "unknown locator label {}", crate::quoted(label))
            }
            Self::UnknownCitationPurpose(purpose) => {
                write!(f, "unknown citation purpose {}", crate::quoted(purpose))
            }
            Self::EmptyCitation => f.write_str("citation requires at least one item"),
            Self::InvalidNoteNumber => f.write_str("note number must be between 1 and 4294967295"),
            Self::Render(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for DocumentError {}

#[derive(Clone)]
/// Prepared library, style, and locale inputs reused across fresh render operations.
pub struct Document {
    library: Arc<Library>,
    style: Arc<PreparedStyle>,
    locale: Option<String>,
}

#[derive(Debug)]
/// Ordered citations and the bibliography produced by one rendering operation.
pub struct RenderedDocument {
    /// One rendered record per request, in request order.
    pub citations: Vec<RenderedRecord>,
    /// Bibliography for the cited entries in style-defined order.
    pub bibliography: RenderedRecord,
}

impl Document {
    /// Prepare shared inputs without retaining citation-processing history.
    #[must_use]
    pub fn new(library: Arc<Library>, style: Arc<PreparedStyle>, locale: Option<String>) -> Self {
        Self {
            library,
            style,
            locale,
        }
    }

    #[must_use]
    /// Return the number of available top-level library entries.
    pub fn entry_count(&self) -> usize {
        self.library.len()
    }

    /// Render the complete ordered request sequence with fresh citation state.
    ///
    /// # Errors
    /// Rejects missing keys, invalid locator or note context, empty citations,
    /// and failures while materializing the renderer's output.
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

    /// Render a bibliography using the supplied citation sequence as context.
    ///
    /// # Errors
    /// Returns citation-input or output-materialization errors as in [`Self::render`].
    #[expect(
        clippy::needless_pass_by_value,
        reason = "The public document operations consistently consume one prepared citation-request sequence, transferred from the host adapter."
    )]
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

    /// Render every library entry in the style's bibliography order.
    ///
    /// # Errors
    /// Returns a rendering or output-materialization failure.
    pub fn full_bibliography(&self) -> Result<RenderedRecord, DocumentError> {
        self.cited_bibliography(full_bibliography_requests(&self.library))
    }
}

#[cfg(test)]
mod tests {
    use crate::{Library, RecoveryPolicy, load_prepared_style};

    use super::*;

    #[test]
    fn citation_purposes_render_author_year_and_prose() {
        let document = test_document(
            "apa",
            "@book{a,author={Doe, Jane},title={A Book},year={2024}}",
        );
        for (purpose, expected) in [
            (CitePurpose::Normal, "(Doe, 2024)"),
            (CitePurpose::Author, "Doe"),
            (CitePurpose::Year, "2024"),
            (CitePurpose::Prose, "Doe (2024)"),
        ] {
            let rendered = document
                .render(vec![CitationRequest::new(
                    vec![Cite::new("a".into(), None, None, purpose)],
                    None,
                )])
                .unwrap();
            assert_eq!(rendered.citations[0].text, expected);
            assert!(rendered.citations[0].html.contains("Doe") || purpose == CitePurpose::Year);
            assert!(rendered.bibliography.text.contains("A Book"));
        }
        let full = document
            .render(vec![CitationRequest::new(
                vec![Cite::new("a".into(), None, None, CitePurpose::Full)],
                None,
            )])
            .unwrap();
        assert!(full.citations[0].text.contains("A Book"));
        assert!(full.citations[0].html.contains("<i>"));
    }

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
                    CitePurpose::Normal,
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
        CitationRequest::new(
            vec![Cite::new(key.to_string(), None, None, CitePurpose::Normal)],
            None,
        )
    }
}
