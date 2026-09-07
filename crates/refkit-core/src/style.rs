use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

use hayagriva::archive;
use hayagriva::citationberg::{IndependentStyle, Style as CslStyle};

use crate::quoted;

mod validate;

use self::validate::{validate_macros, validate_xml_budget};
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleError {
    CachePoisoned,
    DependentStyle(String),
    InvalidXml(String),
    InvalidMacro(String),
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
            Self::InvalidMacro(message) => write!(f, "invalid CSL macro graph: {message}"),
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
}

impl PreparedStyle {
    pub(crate) fn new(inner: IndependentStyle) -> Self {
        let inner = Arc::new(inner);
        Self { inner }
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
    validate_xml_budget(xml)?;
    let style = CslStyle::from_xml(xml)
        .map_err(|err| StyleError::InvalidXml(format!("{err}: {}", err.source)))?;
    prepare_csl_style("xml".to_string(), style)
}

fn prepare_csl_style(id: String, style: CslStyle) -> Result<PreparedStyle, StyleError> {
    match style {
        CslStyle::Independent(style) => {
            validate_macros(&style)?;
            Ok(PreparedStyle::new(style))
        }
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
    fn bundled_styles_pass_preparation_limits() {
        let failures = archive::ArchivedStyle::all()
            .iter()
            .filter_map(|style| {
                let name = style.names()[0];
                load_prepared_style(name)
                    .err()
                    .map(|error| format!("{name}: {error}"))
            })
            .collect::<Vec<_>>();
        assert!(failures.is_empty(), "{}", failures.join("\n"));
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
    fn custom_style_rejects_invalid_macro_dependencies() {
        for (macros, citation, message) in [
            ("", r#"<text macro="missing"/>"#, "missing definition"),
            (
                r#"<macro name="same"><text value="one"/></macro><macro name="same"><text value="two"/></macro>"#,
                r#"<text macro="same"/>"#,
                "duplicate definition",
            ),
            (
                r#"<macro name="a"><text macro="b"/></macro><macro name="b"><text macro="a"/></macro>"#,
                r#"<text macro="a"/>"#,
                "cycle:",
            ),
        ] {
            let error = prepare_style_from_xml(&custom_style(macros, citation)).unwrap_err();
            assert!(
                matches!(error, StyleError::InvalidMacro(ref detail) if detail.contains(message))
            );
        }
    }

    #[test]
    fn shared_macro_dependencies_render_repeated_values() {
        let xml = custom_style(
            r#"<macro name="leaf"><text variable="title"/></macro><macro name="branch"><text macro="leaf"/></macro>"#,
            r#"<group delimiter=" / "><text macro="branch"/><text macro="leaf"/></group>"#,
        );
        let style = prepare_style_from_xml(&xml).unwrap();
        let library =
            crate::Library::parse_biblatex("@book{a,title={Shared}}", crate::RecoveryPolicy::Error)
                .unwrap();
        assert_eq!(
            crate::render_library_citation(&library, "a", &style, None)
                .unwrap()
                .text,
            "Shared / Shared"
        );
    }

    #[test]
    fn custom_style_bounds_macro_expansion() {
        let mut definitions = r#"<macro name="m0"><text value="end"/></macro>"#.to_string();
        for index in 1..66 {
            definitions.push_str(&format!(
                r#"<macro name="m{index}"><text macro="m{}"/></macro>"#,
                index - 1
            ));
        }
        let error = prepare_style_from_xml(&custom_style(&definitions, r#"<text macro="m65"/>"#))
            .unwrap_err();
        assert!(
            matches!(error, StyleError::InvalidMacro(detail) if detail.contains("64 nested elements"))
        );
    }

    #[test]
    fn custom_style_accepts_xml_comments_and_bounded_nesting() {
        let nested = nested_groups(60, r#"<text value="bounded"/>"#);
        let source = custom_style(r#"<!-- <group> is an example -->"#, &nested);
        let style = prepare_style_from_xml(&source).unwrap();
        assert_eq!(style.title(), "Macro contract");
        let library =
            crate::Library::parse_biblatex("@book{a,title={Shared}}", crate::RecoveryPolicy::Error)
                .unwrap();
        assert_eq!(
            crate::render_library_citation(&library, "a", &style, None)
                .unwrap()
                .text,
            "bounded"
        );
    }

    #[test]
    fn macro_expansion_counts_nested_rendering_elements() {
        let branch = nested_groups(30, r#"<text macro="leaf"/>"#);
        let leaf = nested_groups(30, r#"<text value="end"/>"#);
        let definitions =
            format!(r#"<macro name="branch">{branch}</macro><macro name="leaf">{leaf}</macro>"#);
        let source = custom_style(
            &definitions,
            r#"<group><group><text macro="branch"/></group></group>"#,
        );
        let error = prepare_style_from_xml(&source).unwrap_err();
        assert!(
            matches!(error, StyleError::InvalidMacro(detail) if detail.contains("64 nested elements"))
        );
    }

    #[test]
    fn repeated_macro_calls_share_a_bounded_expansion_budget() {
        let mut definitions = r#"<macro name="m0"><text value="end"/></macro>"#.to_string();
        for index in 1..19 {
            let previous = index - 1;
            definitions.push_str(&format!(r#"<macro name="m{index}"><group><text macro="m{previous}"/><text macro="m{previous}"/></group></macro>"#));
        }
        let error = prepare_style_from_xml(&custom_style(&definitions, r#"<text macro="m18"/>"#))
            .unwrap_err();
        assert!(
            matches!(error, StyleError::InvalidMacro(detail) if detail.contains("100000 elements"))
        );
    }

    fn nested_groups(depth: usize, content: &str) -> String {
        format!(
            "{}{content}{}",
            "<group>".repeat(depth),
            "</group>".repeat(depth)
        )
    }

    fn custom_style(macros: &str, citation: &str) -> String {
        format!(
            r#"<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
        <info><title>Macro contract</title><id>https://example.com/macros</id><updated>2026-01-01T00:00:00Z</updated></info>
        {macros}<citation><layout>{citation}</layout></citation></style>"#
        )
    }

    #[test]
    fn invalid_xml_keeps_parser_message() {
        let err = prepare_style_from_xml("<style>").unwrap_err();

        assert!(matches!(err, StyleError::InvalidXml(_)));
    }
}
