use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock};

use hayagriva::archive;
use hayagriva::citationberg::{IndependentStyle, Style as CslStyle};

use crate::quoted;

mod validate;

use self::validate::{validate_macros, validate_xml_budget};
#[derive(Debug, Clone, PartialEq, Eq)]
/// Style lookup, XML validation, or supplied-parent resolution failure.
pub enum StyleError {
    /// A prior failure poisoned the prepared-style cache lock.
    CachePoisoned,
    /// A dependent style requires the named independent parent resource.
    MissingParent(String),
    /// Supplied parent identity or style kind is incompatible with the child.
    InvalidParent(String),
    /// Style XML is invalid or exceeds its structural budget.
    InvalidXml(String),
    /// Macro references form an invalid or oversized expansion graph.
    InvalidMacro(String),
    /// No bundled style has the supplied name or alias.
    UnknownBundledStyle(String),
}

impl fmt::Display for StyleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CachePoisoned => f.write_str("style cache lock is poisoned"),
            Self::MissingParent(id) => write!(f, "supply parent XML for CSL style {}", quoted(id)),
            Self::InvalidParent(message) => write!(f, "invalid CSL parent: {message}"),
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
/// Validated style rules with the requested style's identity and locale precedence.
pub struct PreparedStyle {
    pub(crate) inner: Arc<IndependentStyle>,
    title: String,
    csl_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Catalog metadata for one bundled independent CSL style.
pub struct StyleMetadata {
    /// Canonical name accepted by bundled-style lookup.
    pub name: String,
    /// Additional accepted lookup names.
    pub aliases: Vec<String>,
    /// Human-readable archived style title.
    pub title: String,
    /// CSL style identifier.
    pub csl_id: String,
}

#[must_use]
/// List bundled styles sorted by canonical name.
#[expect(
    clippy::indexing_slicing,
    reason = "The pinned archive's names slice always starts with a canonical name followed by aliases. Catalog tests exercise every archived style through this lookup."
)]
pub fn style_catalog() -> Vec<StyleMetadata> {
    let mut styles: Vec<_> = archive::ArchivedStyle::all()
        .iter()
        .map(|style| StyleMetadata {
            name: style.names()[0].to_string(),
            aliases: style.names()[1..]
                .iter()
                .map(std::string::ToString::to_string)
                .collect(),
            title: style.display_name().to_string(),
            csl_id: style.csl_id().to_string(),
        })
        .collect();
    styles.sort_by(|left, right| left.name.cmp(&right.name));
    styles
}

impl PreparedStyle {
    pub(crate) fn new(inner: IndependentStyle) -> Self {
        let title = inner.info.title.value.clone();
        let csl_id = inner.info.id.clone();
        let inner = Arc::new(inner);
        Self {
            inner,
            title,
            csl_id,
        }
    }

    #[must_use]
    /// Return the requested style's title, including for a dependent style.
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    /// Return the requested style's CSL identity rather than its parent's identity.
    pub fn csl_id(&self) -> &str {
        &self.csl_id
    }
}

/// Load and cache a bundled style by case-insensitive canonical name or alias.
///
/// # Errors
/// Returns an error for an unknown name, a poisoned cache, or invalid archived rules.
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
    let style = prepare_csl_style(archived.get(), None)?;
    let style = Arc::new(style);
    cache
        .lock()
        .map_err(|_| StyleError::CachePoisoned)?
        .insert(key, Arc::clone(&style));
    Ok(style)
}

/// Validate supplied CSL XML and optionally resolve a dependent style's parent.
///
/// # Errors
/// Rejects invalid XML, resource-limit violations, invalid macros, missing parents,
/// or a supplied parent whose kind or CSL identity does not match the child.
pub fn prepare_style_from_xml(
    xml: &str,
    parent_xml: Option<&str>,
) -> Result<PreparedStyle, StyleError> {
    let style = parse_style_xml(xml)?;
    let parent = parent_xml.map(parse_style_xml).transpose()?;
    prepare_csl_style(style, parent)
}

fn parse_style_xml(xml: &str) -> Result<CslStyle, StyleError> {
    validate_xml_budget(xml)?;
    CslStyle::from_xml(xml).map_err(|err| StyleError::InvalidXml(format!("{err}: {}", err.source)))
}

fn prepare_csl_style(
    style: CslStyle,
    parent: Option<CslStyle>,
) -> Result<PreparedStyle, StyleError> {
    match style {
        CslStyle::Independent(style) => {
            if parent.is_some() {
                return Err(StyleError::InvalidParent(
                    "independent style does not accept parent XML".into(),
                ));
            }
            validate_macros(&style)?;
            Ok(PreparedStyle::new(style))
        }
        CslStyle::Dependent(child) => {
            let parent =
                parent.ok_or_else(|| StyleError::MissingParent(child.parent_link.href.clone()))?;
            let CslStyle::Independent(mut parent) = parent else {
                return Err(StyleError::InvalidParent(
                    "parent XML must describe an independent style".into(),
                ));
            };
            if parent.info.id != child.parent_link.href {
                return Err(StyleError::InvalidParent(format!(
                    "expected {}, received {}",
                    quoted(&child.parent_link.href),
                    quoted(&parent.info.id)
                )));
            }
            validate_macros(&parent)?;
            parent.default_locale = child.default_locale.or(parent.default_locale);
            Ok(PreparedStyle {
                inner: Arc::new(parent),
                title: child.info.title.value,
                csl_id: child.info.id,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    #[test]
    fn catalog_exposes_loadable_names_and_aliases_in_sorted_order() {
        let catalog = style_catalog();
        assert!(catalog.windows(2).all(|pair| pair[0].name < pair[1].name));
        let apa = catalog
            .iter()
            .find(|style| style.name == "apa" || style.aliases.iter().any(|alias| alias == "apa"))
            .unwrap();
        assert_eq!(apa.csl_id, "http://www.zotero.org/styles/apa");
        assert!(!apa.title.is_empty());
        let prepared = load_prepared_style(&apa.name).unwrap();
        for alias in &apa.aliases {
            assert_eq!(
                load_prepared_style(alias).unwrap().title(),
                prepared.title()
            );
        }
    }

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
            None,
        )
        .unwrap_err();

        assert_eq!(
            err,
            StyleError::MissingParent("https://example.com/parent".to_string())
        );
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
            let error = prepare_style_from_xml(&custom_style(macros, citation), None).unwrap_err();
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
        let style = prepare_style_from_xml(&xml, None).unwrap();
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
            write!(
                definitions,
                r#"<macro name="m{index}"><text macro="m{}"/></macro>"#,
                index - 1
            )
            .unwrap();
        }
        let error =
            prepare_style_from_xml(&custom_style(&definitions, r#"<text macro="m65"/>"#), None)
                .unwrap_err();
        assert!(
            matches!(error, StyleError::InvalidMacro(detail) if detail.contains("64 nested elements"))
        );
    }

    #[test]
    fn custom_style_accepts_xml_comments_and_bounded_nesting() {
        let nested = nested_groups(60, r#"<text value="bounded"/>"#);
        let source = custom_style(r"<!-- <group> is an example -->", &nested);
        let style = prepare_style_from_xml(&source, None).unwrap();
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
        let error = prepare_style_from_xml(&source, None).unwrap_err();
        assert!(
            matches!(error, StyleError::InvalidMacro(detail) if detail.contains("64 nested elements"))
        );
    }

    #[test]
    fn repeated_macro_calls_share_a_bounded_expansion_budget() {
        let mut definitions = r#"<macro name="m0"><text value="end"/></macro>"#.to_string();
        for index in 1..19 {
            let previous = index - 1;
            write!(definitions, r#"<macro name="m{index}"><group><text macro="m{previous}"/><text macro="m{previous}"/></group></macro>"#).unwrap();
        }
        let error =
            prepare_style_from_xml(&custom_style(&definitions, r#"<text macro="m18"/>"#), None)
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
        let err = prepare_style_from_xml("<style>", None).unwrap_err();

        assert!(matches!(err, StyleError::InvalidXml(_)));
    }
}
