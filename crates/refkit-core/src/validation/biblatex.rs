use ::biblatex::{Bibliography, ChunksExt, RawBibliography};

use super::{
    Identifiers, ValidationCode, ValidationProfile, ValidationReport, ValidationTarget,
    check_identifier, check_url, issue, shared_identifiers,
};
use crate::library::{normalize_reference, parse_error, validate_raw, validate_source};
use crate::{Diagnostic, ParseFailure, RawDocument};

impl RawDocument {
    /// Inspect the current source snapshot using the pinned BibLaTeX field profile.
    pub fn validate(&self) -> Result<ValidationReport, ParseFailure> {
        let source = self
            .render()
            .map_err(|error| Diagnostic::error("syntax_error", None, error))?;
        validate_source(&source)?;
        let snapshot = RawDocument::parse(&source);
        snapshot.resolve()?;
        let raw = RawBibliography::parse(&source).map_err(|error| parse_error(&error))?;
        validate_raw(&raw)?;
        let bibliography =
            Bibliography::from_raw(raw.clone()).map_err(|error| parse_error(&error))?;
        let raw_entries: std::collections::BTreeMap<_, _> = raw
            .entries
            .iter()
            .map(|entry| (entry.v.key.v, entry))
            .collect();
        let mut issues = Vec::new();
        let mut identifiers = Identifiers::new();
        for occurrence in snapshot.entry_occurrences() {
            let Some(entry) = bibliography.get(&occurrence.key) else {
                continue;
            };
            let fields = snapshot
                .field_occurrences(occurrence.id)
                .unwrap_or_default();
            let target = |path: &str| {
                let field = fields
                    .iter()
                    .find(|field| field.name.eq_ignore_ascii_case(path));
                ValidationTarget {
                    entry: occurrence.key.clone(),
                    path: path.into(),
                    entry_id: Some(occurrence.id.index()),
                    field_id: field.map(|field| field.id.index()),
                    span: Some(
                        field.map_or_else(|| occurrence.span.clone(), |field| field.span.clone()),
                    ),
                }
            };
            let verified = entry.verify();
            for field in verified.missing {
                issues.push(issue(
                    ValidationCode::MissingRequiredField,
                    target(field),
                    format!("BibLaTeX profile requires {field} for {}", occurrence.kind),
                ));
            }
            for field in verified.superfluous {
                issues.push(issue(
                    ValidationCode::SuperfluousField,
                    target(field),
                    format!(
                        "BibLaTeX profile does not allow {field} for {}",
                        occurrence.kind
                    ),
                ));
            }
            for (field, error) in verified.malformed {
                issues.push(issue(
                    ValidationCode::MalformedField,
                    target(&field),
                    error.to_string(),
                ));
            }
            for field in ["doi", "isbn", "issn"] {
                if let Some(value) = entry.fields.get(field)
                    && let Some(canonical) = check_identifier(
                        field,
                        &value.format_verbatim(),
                        target(field),
                        &mut issues,
                    )
                {
                    identifiers
                        .entry((field.into(), canonical))
                        .or_default()
                        .push(target(field));
                }
            }
            if let Some(value) = entry.fields.get("url") {
                check_url(&value.format_verbatim(), target("url"), &mut issues);
            }
            if entry
                .fields
                .get("title")
                .is_some_and(|title| title.format_verbatim().trim().is_empty())
            {
                issues.push(issue(
                    ValidationCode::EmptyTitle,
                    target("title"),
                    "Title is present but empty",
                ));
            }
            // xdata is consumed by upstream inheritance, so inspect source references.
            if let Some(raw_entry) = raw_entries.get(occurrence.key.as_str()) {
                for field in &raw_entry.v.fields {
                    let name = field.key.v.to_ascii_lowercase();
                    if !["crossref", "xdata", "xref"].contains(&name.as_str()) {
                        continue;
                    }
                    let keys = normalize_reference(
                        &field.value.v,
                        &raw.abbreviations,
                        name != "crossref",
                    )?;
                    for key in keys {
                        if bibliography.get(&key).is_none() {
                            let mut finding = issue(
                                ValidationCode::UnresolvedReference,
                                target(&name),
                                format!("Reference target {key:?} does not exist"),
                            );
                            finding.related.push(ValidationTarget::record(&key, ""));
                            issues.push(finding);
                        }
                    }
                }
            }
        }
        shared_identifiers(identifiers, &mut issues);
        Ok(ValidationReport {
            profile: ValidationProfile::Biblatex,
            issues,
        })
    }
}
