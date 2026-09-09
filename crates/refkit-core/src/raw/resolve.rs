use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;

use biblatex::{Field, RawChunk, Spanned};

use super::parse::{parse_assignment_atoms, parse_raw_document};
use super::{RawBlock, RawDocument, RawValueAtom, RawValueMode, ResolvedBibEntry};
use crate::library::{
    Diagnostic, FieldResolver, ParseFailure, validate_source, validate_source_size,
};

pub(super) fn resolve_document(
    document: &RawDocument,
) -> Result<Vec<ResolvedBibEntry>, ParseFailure> {
    let (changed, edited_bytes) = document
        .data
        .entry_blocks
        .iter()
        .flat_map(|entry| &entry.field_blocks)
        .filter(|field| field.changed)
        .fold((false, 0usize), |(_, bytes), field| {
            (true, bytes.saturating_add(field.value.len()))
        });
    validate_source_size(edited_bytes)?;
    let current;
    let data = if changed {
        let source = document
            .render()
            .map_err(|message| Diagnostic::error("syntax_error", None, message))?;
        validate_source(&source)?;
        current = parse_raw_document(&source);
        &current
    } else {
        validate_source_size(
            document
                .data
                .blocks
                .last()
                .map_or(0, |block| block.span().end),
        )?;
        &document.data
    };
    let mut definitions = Vec::new();
    for block in &data.blocks {
        match block {
            RawBlock::StringDef { raw, span, .. } => {
                let body_start = raw.find(['{', '(']).expect("parsed block has an opener") + 1;
                let (key, _, atoms) = parse_assignment_atoms(&raw[body_start..raw.len() - 1])
                    .ok_or_else(|| {
                        Diagnostic::error(
                            "syntax_error",
                            Some(span.clone()),
                            "invalid BibTeX string definition".to_string(),
                        )
                    })?;
                definitions.push((key, atoms, span.clone()));
            }
            RawBlock::Failed { error, span, .. } => {
                return Err(
                    Diagnostic::error("syntax_error", Some(span.clone()), error.clone()).into(),
                );
            }
            _ => {}
        }
    }
    let definitions: Vec<_> = definitions
        .iter()
        .map(|(key, atoms, span)| (key, as_field(atoms, span)))
        .collect();
    let abbreviations: HashMap<_, _> = definitions
        .iter()
        .map(|(key, value)| ((*key).clone(), value))
        .collect();
    let mut resolver = FieldResolver::new(abbreviations);
    let mut keys = HashSet::new();
    let mut entries = Vec::with_capacity(data.entry_blocks.len());
    for entry in &data.entry_blocks {
        if entry.key.is_empty() || !keys.insert(&entry.key) {
            let code = if entry.key.is_empty() {
                "syntax_error"
            } else {
                "duplicate_key"
            };
            let mut diagnostic = Diagnostic::error(
                code,
                Some(entry.span.clone()),
                format!(
                    "BibTeX entry key {:?} must be nonempty and unique",
                    entry.key
                ),
            );
            diagnostic.entry = Some(entry.key.clone());
            return Err(diagnostic.into());
        }
        let mut fields = BTreeMap::new();
        for field in &entry.field_blocks {
            let name = field.name.to_ascii_lowercase();
            let result = if fields.contains_key(&name) {
                Err(Diagnostic::error(
                    "duplicate_field",
                    Some(field.span.clone()),
                    format!("duplicate BibTeX field {name:?}"),
                ))
            } else if field.value_mode == RawValueMode::Missing {
                Err(Diagnostic::error(
                    "syntax_error",
                    Some(field.span.clone()),
                    format!("BibTeX field {name:?} requires an assignment"),
                ))
            } else {
                resolver.resolve(&as_field(&field.value_atoms, &field.span))
            };
            let value = result.map_err(|mut diagnostic| {
                diagnostic.entry = Some(entry.key.clone());
                diagnostic.field = Some(name.clone());
                diagnostic
            })?;
            fields.insert(name, value);
        }
        entries.push(ResolvedBibEntry {
            key: entry.key.clone(),
            entry_type: entry.kind.to_ascii_lowercase(),
            fields,
        });
    }
    Ok(entries)
}

fn as_field<'a>(atoms: &'a [RawValueAtom], span: &Range<usize>) -> Field<'a> {
    atoms
        .iter()
        .map(|atom| {
            let value = if atom.value_mode == RawValueMode::Bare
                && !atom.value.bytes().all(|byte| byte.is_ascii_digit())
            {
                RawChunk::Abbreviation(&atom.value)
            } else {
                RawChunk::Normal(&atom.value)
            };
            Spanned::new(value, span.clone())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_source_fields_with_macros_and_preserves_tex() {
        let source = r#"@string{publisher = "Example Press"}
@string{URL = "https://example.test/"}
@CustomType(Work, TITLE = {A {Protected} \LaTeX{} title},
  HOWPUBLISHED = url # {paper}, publisher = PUBLISHER, year = 2026,
  custom = {  keep spaces  }, month = SEP, crossref = {Parent})
@book{Parent, title = {Parent Title}}"#;
        let document = RawDocument::parse(source);
        let entries = document.resolve().unwrap();
        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            ["Work", "Parent"]
        );
        assert_eq!(entries[0].entry_type, "customtype");
        assert_eq!(
            entries[0].fields,
            BTreeMap::from([
                (
                    "title".to_string(),
                    r"A {Protected} \LaTeX{} title".to_string()
                ),
                (
                    "howpublished".to_string(),
                    "https://example.test/paper".to_string()
                ),
                ("publisher".to_string(), "Example Press".to_string()),
                ("year".to_string(), "2026".to_string()),
                ("custom".to_string(), "  keep spaces  ".to_string()),
                ("month".to_string(), "September".to_string()),
                ("crossref".to_string(), "Parent".to_string()),
            ])
        );
        assert_eq!(document.render().unwrap(), source);
    }

    #[test]
    fn resolution_observes_edits_and_matches_rendered_readback() {
        let mut document = RawDocument::parse(
            r#"@string{prefix="old"}@misc{work,title=prefix # { title},year=2025}"#,
        );
        let entry = document.unique_entry("work").unwrap().unwrap();
        let field = document.unique_field(entry, "title").unwrap().unwrap();
        document
            .set_field_value(entry, field, r#"new # "literal""#.to_string())
            .unwrap();
        let before = document.render().unwrap();
        let entries = document.resolve().unwrap();
        assert_eq!(entries[0].fields["title"], r#"new # "literal""#);
        assert_eq!(entries, RawDocument::parse(&before).resolve().unwrap());
        assert_eq!(document.render().unwrap(), before);
    }

    #[test]
    fn reference_fields_resolve_values_and_preserve_authored_fields() {
        let entries = RawDocument::parse(
            r#"@string{parent="P"}@book{P,title={Parent}}
@misc{child,crossref=parent,xdata=parent # {, X},note={own}}"#,
        )
        .resolve()
        .unwrap();
        assert_eq!(
            entries[1].fields,
            BTreeMap::from([
                ("crossref".to_string(), "P".to_string()),
                ("xdata".to_string(), "P, X".to_string()),
                ("note".to_string(), "own".to_string()),
            ])
        );
    }

    #[test]
    fn macro_lookup_uses_final_definitions_and_preserves_literal_escapes() {
        let entries = RawDocument::parse(
            r#"@misc{Work,title=pub,month=JAN,note={A \} B}}
@misc{work,title={lowercase key}}
@string{PUB={old}}@string{pub=next,}@string{next={new}}@string{jan={Custom Month}}"#,
        )
        .resolve()
        .unwrap();
        assert_eq!(entries[0].fields["title"], "new");
        assert_eq!(entries[0].fields["month"], "Custom Month");
        assert_eq!(entries[0].fields["note"], r"A \} B");
        assert_eq!(entries[1].key, "work");
    }

    #[test]
    fn bounds_source_and_edited_input_size() {
        let source = " ".repeat(16 * 1024 * 1024 + 1);
        assert_eq!(
            RawDocument::parse(&source)
                .resolve()
                .unwrap_err()
                .diagnostics[0]
                .code,
            "resource_limit"
        );
        let mut document = RawDocument::parse("@misc{work,title={old}}");
        let entry = document.unique_entry("work").unwrap().unwrap();
        let field = document.unique_field(entry, "title").unwrap().unwrap();
        document.set_field_value(entry, field, source).unwrap();
        assert_eq!(
            document.resolve().unwrap_err().diagnostics[0].code,
            "resource_limit"
        );
    }

    #[test]
    fn resolves_edits_that_bring_source_within_the_size_limit() {
        let source = format!("@misc{{work,title={{{}}}}}", "x".repeat(16 * 1024 * 1024));
        let mut document = RawDocument::parse(&source);
        let entry = document.unique_entry("work").unwrap().unwrap();
        let field = document.unique_field(entry, "title").unwrap().unwrap();
        document
            .set_field_value(entry, field, "small".to_string())
            .unwrap();

        let entries = document.resolve().unwrap();
        assert_eq!(entries[0].fields["title"], "small");
        assert_eq!(
            entries,
            RawDocument::parse(&document.render().unwrap())
                .resolve()
                .unwrap()
        );
    }

    #[test]
    fn nesting_failures_have_valid_source_spans_for_unicode_expressions() {
        let source = format!(
            "@string{{accent=\"é\"}}@misc{{work,title=accent # {{{}é{}}}}}",
            "{".repeat(64),
            "}".repeat(64),
        );
        let failure = RawDocument::parse(&source).resolve().unwrap_err();
        let diagnostic = &failure.diagnostics[0];
        assert_eq!(diagnostic.code, "resource_limit");
        assert_eq!(diagnostic.entry.as_deref(), Some("work"));
        assert_eq!(diagnostic.field.as_deref(), Some("title"));
        assert!(
            diagnostic
                .span
                .as_ref()
                .is_some_and(|span| source.get(span.clone()).is_some())
        );
    }

    #[test]
    fn rejects_ambiguous_fields_and_invalid_dependencies() {
        for (source, code) in [
            ("@misc{work,title=missing}", "unknown_abbreviation"),
            (
                "@string{a=B}@string{b=A}@misc{work,title=a}",
                "cyclic_abbreviation",
            ),
            ("@misc{work,title={a},TITLE={b}}", "duplicate_field"),
        ] {
            let failure = RawDocument::parse(source).resolve().unwrap_err();
            let diagnostic = &failure.diagnostics[0];
            assert_eq!(diagnostic.code, code);
            assert_eq!(diagnostic.entry.as_deref(), Some("work"));
            assert_eq!(diagnostic.field.as_deref(), Some("title"));
            assert!(
                diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| source.get(span.clone()).is_some())
            );
        }
        for (source, code) in [
            ("@misc{work}@misc{work}", "duplicate_key"),
            ("@misc{work,title={unfinished", "syntax_error"),
            ("@string{a=}", "syntax_error"),
        ] {
            assert_eq!(
                RawDocument::parse(source)
                    .resolve()
                    .unwrap_err()
                    .diagnostics[0]
                    .code,
                code
            );
        }
    }

    #[test]
    fn bounds_aggregate_expansion_and_dependency_depth() {
        let mut source = "@string{x0={}}".to_string();
        for index in 1..20 {
            source.push_str(&format!(
                "@string{{x{index}=x{} # x{}}}",
                index - 1,
                index - 1
            ));
        }
        source.push_str("@misc{work,title=x19}");
        assert_eq!(
            RawDocument::parse(&source)
                .resolve()
                .unwrap_err()
                .diagnostics[0]
                .code,
            "resource_limit"
        );

        let mut source = "@string{x0={ok}}".to_string();
        for index in 1..65 {
            source.push_str(&format!("@string{{x{index}=x{}}}", index - 1));
        }
        source.push_str("@misc{work,title=x64}");
        assert_eq!(
            RawDocument::parse(&source)
                .resolve()
                .unwrap_err()
                .diagnostics[0]
                .code,
            "resource_limit"
        );

        let source = format!(
            "@string{{a={{{}}}}}@misc{{work,a=a,b=a,c=a}}",
            "x".repeat(6 * 1024 * 1024)
        );
        assert_eq!(
            RawDocument::parse(&source)
                .resolve()
                .unwrap_err()
                .diagnostics[0]
                .code,
            "resource_limit"
        );
    }
}
