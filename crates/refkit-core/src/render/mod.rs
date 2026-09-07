mod bibliography;
mod citation;
mod html;
mod text;

use std::sync::OnceLock;

use hayagriva::BufWriteFormat;
use hayagriva::archive;
use hayagriva::citationberg::Locale as CslLocale;

use crate::{DocumentError, Library, PreparedStyle};

pub(crate) use self::bibliography::bibliography_to_text_html;
use self::citation::request_for_keys;
pub(crate) use self::citation::{full_bibliography_requests, process_citations};
pub(crate) use self::html::{elem_children_to_html, safe_href};
pub(crate) use self::text::elem_children_to_string;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedOutput {
    pub text: String,
    pub html: String,
}

pub(crate) fn bundled_locales() -> &'static [CslLocale] {
    static LOCALES: OnceLock<Vec<CslLocale>> = OnceLock::new();
    LOCALES.get_or_init(archive::locales).as_slice()
}

pub fn is_bundled_locale(code: &str) -> bool {
    bundled_locales()
        .iter()
        .any(|locale| locale.lang.as_ref().is_some_and(|lang| lang.0 == code))
}

pub fn render_library_citation(
    library: &Library,
    key: &str,
    style: &PreparedStyle,
    locale: Option<&str>,
) -> Result<RenderedOutput, DocumentError> {
    render_library_citation_group(library, &[key], style, locale)
}

pub fn render_library_citation_each(
    library: &Library,
    keys: &[&str],
    style: &PreparedStyle,
    locale: Option<&str>,
) -> Result<Vec<RenderedOutput>, DocumentError> {
    let requests = keys
        .iter()
        .map(|key| request_for_keys(&[key]))
        .collect::<Vec<_>>();
    let rendered = process_citations(library, style, locale, &requests)?;
    rendered.citations.iter().map(citation_output).collect()
}

pub fn render_library_citation_group(
    library: &Library,
    keys: &[&str],
    style: &PreparedStyle,
    locale: Option<&str>,
) -> Result<RenderedOutput, DocumentError> {
    let rendered = process_citations(library, style, locale, &[request_for_keys(keys)])?;
    citation_output(&rendered.citations[0])
}

pub fn render_library_bibliography(
    library: &Library,
    style: &PreparedStyle,
    locale: Option<&str>,
) -> Result<RenderedOutput, DocumentError> {
    let rendered = process_citations(library, style, locale, &full_bibliography_requests(library))?;
    let (text, html) = match rendered.bibliography {
        Some(bibliography) => bibliography_to_text_html(&bibliography),
        None => Ok((String::new(), String::new())),
    }
    .map_err(DocumentError::Render)?;
    Ok(RenderedOutput { text, html })
}

fn citation_output(
    citation: &hayagriva::RenderedCitation,
) -> Result<RenderedOutput, DocumentError> {
    Ok(RenderedOutput {
        text: elem_children_to_string(&citation.citation, BufWriteFormat::Plain)
            .map_err(DocumentError::Render)?,
        html: elem_children_to_html(&citation.citation).map_err(DocumentError::Render)?,
    })
}

#[cfg(test)]
mod tests {
    use hayagriva::{ElemChild, ElemChildren};

    use crate::load_prepared_style;

    use super::*;

    #[test]
    fn safe_href_allows_only_link_schemes() {
        assert_eq!(
            safe_href(" https://example.com/path "),
            Some("https://example.com/path")
        );
        assert_eq!(safe_href("http://example.com"), Some("http://example.com"));
        assert_eq!(
            safe_href("mailto:dev@example.com"),
            Some("mailto:dev@example.com")
        );
        assert_eq!(safe_href("javascript:alert(1)"), None);
        assert_eq!(safe_href("data:text/html,hello"), None);
        assert_eq!(safe_href("/relative:with-colon"), None);
        assert_eq!(safe_href(""), None);
    }

    #[test]
    fn html_renderer_escapes_special_characters() {
        let children = ElemChildren(vec![ElemChild::Text(formatted(
            "A&B <tag> \"quote\" 'apostrophe'",
        ))]);

        assert_eq!(
            elem_children_to_html(&children).unwrap(),
            "A&amp;B &lt;tag&gt; &quot;quote&quot; &#39;apostrophe&#39;"
        );
    }

    #[test]
    fn html_renderer_suppresses_unsafe_links() {
        let children = ElemChildren(vec![ElemChild::Link {
            text: formatted("link"),
            url: "javascript:alert(1)".to_string(),
        }]);

        assert_eq!(elem_children_to_html(&children).unwrap(), "link");
    }

    #[test]
    fn renders_citation_text_and_html() {
        let library = Library::parse_biblatex(
            "@article{doe2024, author = {Doe, Jane}, title = {Core}, year = {2024}}",
            crate::RecoveryPolicy::Report,
        )
        .unwrap();
        let style = load_prepared_style("apa").unwrap();

        let rendered =
            render_library_citation(&library, "doe2024", style.as_ref(), Some("en-US")).unwrap();

        assert!(rendered.text.contains("Doe"));
        assert!(rendered.html.contains("Doe"));
    }

    #[test]
    fn renders_citation_each_in_key_order() {
        let library = Library::parse_biblatex(
            "@article{doe2024, author = {Doe, Jane}, title = {Core}, year = {2024}}
             @article{roe2023, author = {Roe, Richard}, title = {Edges}, year = {2023}}",
            crate::RecoveryPolicy::Report,
        )
        .unwrap();
        let style = load_prepared_style("apa").unwrap();

        let rendered = render_library_citation_each(
            &library,
            &["doe2024", "roe2023"],
            style.as_ref(),
            Some("en-US"),
        )
        .unwrap();

        assert_eq!(rendered.len(), 2);
        assert!(rendered[0].text.contains("Doe"));
        assert!(rendered[1].text.contains("Roe"));
        assert!(rendered[0].html.contains("Doe"));
        assert!(rendered[1].html.contains("Roe"));
    }

    #[test]
    fn citation_each_disambiguates_same_author_and_year() {
        let library = Library::parse_biblatex(
            "@article{doe2024a, author = {Doe, Jane}, title = {Alpha}, year = {2024}}
             @article{doe2024b, author = {Doe, Jane}, title = {Beta}, year = {2024}}",
            crate::RecoveryPolicy::Report,
        )
        .unwrap();
        let style = load_prepared_style("apa").unwrap();

        let rendered = render_library_citation_each(
            &library,
            &["doe2024a", "doe2024b"],
            style.as_ref(),
            Some("en-US"),
        )
        .unwrap();

        assert_eq!(rendered.len(), 2);
        assert_ne!(rendered[0].text, rendered[1].text);
    }

    fn formatted(text: &str) -> hayagriva::Formatted {
        hayagriva::Formatted {
            text: text.to_string(),
            formatting: hayagriva::Formatting::default(),
        }
    }
}
