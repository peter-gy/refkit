use super::recovery::recover_biblatex_library;
use super::{ParseReport, ParsedLibrary, RecoveryPolicy};

pub(super) fn parse_biblatex_library(
    source: &str,
    recovery: RecoveryPolicy,
) -> Result<ParsedLibrary, String> {
    let parsed = match recovery {
        RecoveryPolicy::Error => parse_biblatex_strict(source),
        RecoveryPolicy::Report => recover_biblatex_library(source, true),
    }?;
    reject_recovered_empty_bibtex(source, parsed)
}

pub(super) fn parse_hayagriva_yaml(source: &str) -> Result<ParsedLibrary, String> {
    hayagriva::io::from_yaml_str(source)
        .map(|inner| ParsedLibrary {
            inner,
            diagnostics: Vec::new(),
        })
        .map_err(|err| format!("yaml parse error: {err}"))
}

pub fn parse_bibtex_report(source: &str, recovery: RecoveryPolicy) -> ParseReport {
    match parse_biblatex_library(source, recovery) {
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
    parsed: ParsedLibrary,
) -> Result<ParsedLibrary, String> {
    if recovered_empty_source_is_failure(source, &parsed) {
        return Err(parsed.diagnostics.join("\n"));
    }
    if parsed.inner.is_empty() && !source.trim().is_empty() && parsed.diagnostics.is_empty() {
        let diagnosed = recover_biblatex_library(source, true)?;
        if recovered_empty_source_is_failure(source, &diagnosed) {
            return Err(diagnosed.diagnostics.join("\n"));
        }
    }
    Ok(parsed)
}

fn parse_biblatex_strict(source: &str) -> Result<ParsedLibrary, String> {
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
