use std::path::PathBuf;

use crate::quoted;

use super::read::read_bibliography_text;
use super::recovery::recover_biblatex_library;
use super::{ParseReport, ParsedLibrary};

pub(super) fn parse_library_path(
    path: PathBuf,
    strict: bool,
    diagnostics: bool,
) -> Result<ParsedLibrary, String> {
    let text = read_bibliography_text(&path)?;
    let mut parsed = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("bib") => parse_library_source(&text.source, "bibtex", strict, diagnostics),
        Some("yaml" | "yml") => parse_library_source(&text.source, "yaml", strict, diagnostics),
        Some(ext) => Err(format!(
            "unsupported bibliography extension {}",
            quoted(ext)
        )),
        None => Err("bibliography path has no extension".to_string()),
    }?;
    if diagnostics && let Some(diagnostic) = text.diagnostic {
        parsed.diagnostics.insert(0, diagnostic);
    }
    Ok(parsed)
}

pub(super) fn parse_library_source(
    source: &str,
    format: &str,
    strict: bool,
    diagnostics: bool,
) -> Result<ParsedLibrary, String> {
    match format.to_ascii_lowercase().as_str() {
        "bib" | "bibtex" | "biblatex" => {
            let parsed = parse_biblatex_library(source, strict, diagnostics)?;
            reject_recovered_empty_bibtex(source, strict, parsed)
        }
        "yaml" | "yml" => hayagriva::io::from_yaml_str(source)
            .map(|inner| ParsedLibrary {
                inner,
                diagnostics: Vec::new(),
            })
            .map_err(|err| format!("yaml parse error: {err}")),
        other => Err(format!("unsupported bibliography format {}", quoted(other))),
    }
}

pub(super) fn parse_bibtex_value_source(
    source: &str,
    strict: bool,
) -> Result<ParsedLibrary, String> {
    parse_library_source(source, "bibtex", strict, false)
}

pub fn parse_bibtex_report_source(source: &str, strict: bool) -> ParseReport {
    match parse_library_source(source, "bibtex", strict, true) {
        Ok(parsed) if recovered_empty_source_is_failure(source, &parsed) => ParseReport {
            ok: false,
            entry_count: None,
            keys: None,
            diagnostics: parsed.diagnostics,
        },
        Ok(parsed) => ParseReport {
            ok: true,
            entry_count: Some(parsed.inner.len()),
            keys: Some(parsed.inner.keys().map(str::to_string).collect()),
            diagnostics: parsed.diagnostics,
        },
        Err(err) => ParseReport {
            ok: false,
            entry_count: None,
            keys: None,
            diagnostics: vec![err],
        },
    }
}

fn recovered_empty_source_is_failure(source: &str, parsed: &ParsedLibrary) -> bool {
    !source.trim().is_empty() && parsed.inner.is_empty() && !parsed.diagnostics.is_empty()
}

fn reject_recovered_empty_bibtex(
    source: &str,
    strict: bool,
    parsed: ParsedLibrary,
) -> Result<ParsedLibrary, String> {
    if recovered_empty_source_is_failure(source, &parsed) {
        return Err(parsed.diagnostics.join("\n"));
    }
    if parsed.inner.is_empty() && !source.trim().is_empty() && parsed.diagnostics.is_empty() {
        let diagnosed = parse_biblatex_library(source, strict, true)?;
        if recovered_empty_source_is_failure(source, &diagnosed) {
            return Err(diagnosed.diagnostics.join("\n"));
        }
    }
    Ok(parsed)
}

fn parse_biblatex_library(
    source: &str,
    strict: bool,
    diagnostics: bool,
) -> Result<ParsedLibrary, String> {
    if !strict {
        return recover_biblatex_library(source, diagnostics);
    }

    match hayagriva::io::from_biblatex_str(source) {
        Ok(inner) => Ok(ParsedLibrary {
            inner,
            diagnostics: Vec::new(),
        }),
        Err(errors) => Err(format_biblatex_errors(&errors)),
    }
}

fn format_biblatex_errors(errors: &[hayagriva::io::BibLaTeXError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}
