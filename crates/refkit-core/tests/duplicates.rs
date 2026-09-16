//! Reviewable duplicate evidence and explicit source-preserving merge plans.

#![cfg(test)]

use refkit_core::{
    DuplicateConflictKind, DuplicateRule, MergeErrorCode, MergeFieldChoice, MergeRequest,
    RawDocument,
};
use std::fmt::Write as _;

#[test]
fn abstract_signatures_limit_normalized_unicode_characters_before_the_suffix() {
    let mut source = String::new();
    for (key, prefix, suffix) in [
        ("a", "İ, ".repeat(50), "x".repeat(100_000)),
        ("b", "İ-".repeat(50), "y".repeat(100_000)),
        ("c", "i".repeat(100), String::new()),
    ] {
        writeln!(source, "@misc{{{key},abstract={{{prefix}{suffix}}}}}").unwrap();
    }
    let document = RawDocument::parse(&source);
    let report = document
        .find_duplicates(Some(&[DuplicateRule::Abstract]))
        .unwrap();
    assert_eq!(report.groups.len(), 1);
    let group = &report.groups[0];
    assert_eq!(
        group
            .members
            .iter()
            .map(|member| member.key.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert_eq!(group.evidence[0].signature, "i\u{307}".repeat(50));
    assert_eq!(group.evidence[0].signature.chars().count(), 100);
}

#[test]
fn reverse_bridge_groups_retain_the_first_occurrence_across_review_and_tidy() {
    let count = 256;
    let mut source = String::new();
    for index in 0..count {
        writeln!(
            source,
            "@misc{{n{index},doi={{10.1000/{index}}},abstract={{abstract{index}}}}}"
        )
        .unwrap();
    }
    for index in (1..count).rev() {
        writeln!(
            source,
            "@misc{{bridge{index},doi={{10.1000/{index}}},abstract={{abstract{}}}}}",
            index - 1
        )
        .unwrap();
    }
    writeln!(source, "@misc{{child,crossref={{n{}}}}}", count - 1).unwrap();
    let rules = [DuplicateRule::Doi, DuplicateRule::Abstract];
    let document = RawDocument::parse(&source);
    let report = document.find_duplicates(Some(&rules)).unwrap();
    assert_eq!(report.groups.len(), 1);
    let group = &report.groups[0];
    assert_eq!(group.id.index(), 0);
    assert_eq!(group.members.len(), 2 * count - 1);
    assert_eq!(group.evidence.len(), 2 * (count - 1));
    assert!(
        group
            .members
            .iter()
            .enumerate()
            .all(|(index, member)| member.entry_id.index() == index)
    );

    let tidy = refkit_core::tidy_bibtex(
        &source,
        refkit_core::TidyOptions {
            duplicates: Some(rules.to_vec()),
            merge: Some(refkit_core::MergeStrategy::First),
            ..refkit_core::TidyOptions::default()
        },
    )
    .unwrap();
    assert_eq!(tidy.warnings.len(), 2 * (count - 1));
    let (warning_pairs, remainder) = tidy.warnings.as_chunks::<2>();
    assert!(remainder.is_empty());
    for [doi, abstract_match] in warning_pairs {
        assert_eq!(doi.rule(), Some(DuplicateRule::Doi));
        assert_eq!(abstract_match.rule(), Some(DuplicateRule::Abstract));
    }
    let output = RawDocument::parse(&tidy.bibtex);
    assert_eq!(output.entry_keys(), ["n0", "child"]);
    let child = output.unique_entry("child").unwrap().unwrap();
    let reference = output.unique_field(child, "crossref").unwrap().unwrap();
    assert_eq!(output.field_info(child, reference).unwrap().value, "n0");
}

#[test]
fn review_groups_expose_rule_signatures_and_identifier_conflicts() {
    let document = RawDocument::parse(
        "@article{a,title={Study},author={Doe, Jane},doi={10.1234/a-b}}@article{b,title={Study},author={Doe, Jane},doi={10.1234/ab}}",
    );
    let report = document
        .find_duplicates(Some(&[
            DuplicateRule::Doi,
            DuplicateRule::Citation,
            DuplicateRule::Doi,
        ]))
        .unwrap();
    assert_eq!(report.rules, [DuplicateRule::Doi, DuplicateRule::Citation]);
    assert_eq!(report.groups.len(), 1);
    let group = &report.groups[0];
    assert_eq!(group.members.len(), 2);
    assert_eq!(group.evidence.len(), 2);
    assert_eq!(group.evidence[0].signature, "101234ab");
    assert!(group.conflicts.iter().any(|conflict| conflict.kind
        == DuplicateConflictKind::Identifier
        && conflict.field == "doi"));
    assert_eq!(
        document
            .find_duplicates(Some(&[DuplicateRule::Doi, DuplicateRule::Citation]))
            .unwrap(),
        report
    );
}

#[test]
fn reviewed_field_choices_compile_to_expression_preserving_patches() {
    let source = "@string{press={Example Press}}\n@book{a,title={First},doi={10.1234/work}}\n@book{b,title={Second},doi={10.1234/work},publisher=press,url={https://example.org/a%2Fb},note={Line one\nLine two}}\n@misc{child,crossref={b}}";
    let document = RawDocument::parse(source);
    let a = document.unique_entry("a").unwrap().unwrap();
    let b = document.unique_entry("b").unwrap().unwrap();
    let mut request = MergeRequest {
        entries: vec![a, b],
        retain: a,
        fields: vec![],
        entry_type: None,
    };
    let undecided = document.plan_merge(&request).unwrap();
    assert!(undecided.patch.is_none());
    assert_eq!(
        undecided
            .conflicts
            .iter()
            .map(|conflict| conflict.field.as_str())
            .collect::<Vec<_>>(),
        ["title"]
    );
    request.fields.push(MergeFieldChoice::Take {
        name: "title".into(),
        entry_id: b,
        field_id: document.unique_field(b, "title").unwrap().unwrap(),
    });
    let plan = document.plan_merge(&request).unwrap();
    let result = document.apply_patch(plan.patch.as_ref().unwrap()).unwrap();
    assert_eq!(document.render().unwrap(), source);
    assert_eq!(result.document.entry_keys(), ["a", "child"]);
    let resolved = result.document.resolve().unwrap();
    assert_eq!(resolved[0].fields["title"], "Second");
    assert_eq!(resolved[0].fields["publisher"], "Example Press");
    assert_eq!(resolved[0].fields["url"], "https://example.org/a%2Fb");
    assert_eq!(resolved[0].fields["note"], "Line one\nLine two");
    assert_eq!(resolved[1].fields["crossref"], "a");
    assert!(
        result
            .document
            .render()
            .unwrap()
            .contains("publisher = press")
    );
}

#[test]
fn review_and_merge_preserve_duplicate_field_occurrences_and_expressions() {
    let source = "@string{prefix={Selected}}\n@book{a,title={First},TITLE=prefix # { tail},doi={10.1234/work}}\n@book{b,title={Other},doi={10.1234/work}}";
    let document = RawDocument::parse(source);
    let report = document
        .find_duplicates(Some(&[DuplicateRule::Doi]))
        .unwrap();
    let title = report.groups[0]
        .conflicts
        .iter()
        .find(|conflict| conflict.field == "title")
        .unwrap();
    assert_eq!(
        title
            .values
            .iter()
            .map(|value| (
                value.entry_id.index(),
                value.field_id.unwrap().index(),
                value.expression.as_str(),
            ))
            .collect::<Vec<_>>(),
        [
            (0, 0, "{First}"),
            (0, 1, "prefix # { tail}"),
            (1, 0, "{Other}"),
        ]
    );
    let chosen = &title.values[1];
    let plan = document
        .plan_merge(&MergeRequest {
            entries: report.groups[0]
                .members
                .iter()
                .map(|member| member.entry_id)
                .collect(),
            retain: chosen.entry_id,
            fields: vec![MergeFieldChoice::Take {
                name: "title".into(),
                entry_id: chosen.entry_id,
                field_id: chosen.field_id.unwrap(),
            }],
            entry_type: None,
        })
        .unwrap();
    let result = document.apply_patch(plan.patch.as_ref().unwrap()).unwrap();
    assert_eq!(document.render().unwrap(), source);
    assert_eq!(result.document.entry_keys(), ["a"]);
    assert!(
        result
            .document
            .render()
            .unwrap()
            .contains("TITLE=prefix # { tail}")
    );
    assert_eq!(
        result.document.resolve().unwrap()[0].fields["title"],
        "Selected tail"
    );
}

#[test]
fn type_and_field_conflicts_require_explicit_choices() {
    let document =
        RawDocument::parse("@book{a,title={A},note={Keep}}@article{b,title={A},note={Discard}}");
    let ids = document.entry_occurrences();
    let mut request = MergeRequest {
        entries: ids.iter().map(|entry| entry.id).collect(),
        retain: ids[0].id,
        fields: vec![],
        entry_type: None,
    };
    assert_eq!(document.plan_merge(&request).unwrap().conflicts.len(), 2);
    request.entry_type = Some("book".into());
    request.fields.push(MergeFieldChoice::Drop {
        name: "note".into(),
    });
    let plan = document.plan_merge(&request).unwrap();
    let result = document.apply_patch(plan.patch.as_ref().unwrap()).unwrap();
    assert_eq!(result.document.resolve().unwrap()[0].fields.len(), 1);
    request.fields.push(MergeFieldChoice::Drop {
        name: "note".into(),
    });
    assert_eq!(
        document.plan_merge(&request).unwrap_err().code,
        MergeErrorCode::InvalidChoice
    );
}

#[test]
fn merge_rejects_ambiguous_removals_and_reference_cycles() {
    for source in [
        "@book{a,title={A}}@book{b,title={A}}@book{b,title={Outside}}@misc{c,crossref={b}}",
        "@book{a,title={A},crossref={b}}@book{b,title={A}}",
        "@book{a,title={A},crossref={c}}@book{b,title={A}}@book{c,crossref={b}}@book{c,title={C}}",
    ] {
        let document = RawDocument::parse(source);
        let ids = document.entry_occurrences();
        let request = MergeRequest {
            entries: vec![ids[0].id, ids[1].id],
            retain: ids[0].id,
            fields: vec![],
            entry_type: None,
        };
        let error = document.plan_merge(&request).unwrap_err();
        assert!(matches!(
            error.code,
            MergeErrorCode::AmbiguousReference | MergeErrorCode::ReferenceCycle
        ));
        assert_eq!(document.render().unwrap(), source);
    }
}

#[test]
fn expression_edits_validate_whole_values_before_changing_source() {
    let document = RawDocument::parse("@string{p={Press}}@book{a,publisher={Old}}");
    let entry = document.entry_occurrences()[0].id;
    let field = document.unique_field(entry, "publisher").unwrap().unwrap();
    let edit = |value: &str| refkit_core::BibEdit::SetField {
        entry_id: entry,
        field_id: field,
        value: value.into(),
        expression: true,
    };
    let result = document.apply_patch(&[edit("p # { Supplement}")]).unwrap();
    assert_eq!(
        result.document.resolve().unwrap()[0].fields["publisher"],
        "Press Supplement"
    );
    for value in ["", "{A}, injected={Bad}", "{A}} @book{bad", "p #"] {
        assert!(document.apply_patch(&[edit(value)]).is_err(), "{value}");
    }
}

#[test]
fn xref_rewrites_do_not_create_inheritance_edges() {
    let source = "@book{a,title={A},xref={c}}@book{b,title={A}}@misc{c,title={C},xref={b}}";
    let document = RawDocument::parse(source);
    let entries = document.entry_occurrences();
    let request = MergeRequest {
        entries: vec![entries[0].id, entries[1].id],
        retain: entries[0].id,
        fields: vec![],
        entry_type: None,
    };
    let plan = document.plan_merge(&request).unwrap();
    let merged = document
        .apply_patch(plan.patch.as_ref().unwrap())
        .unwrap()
        .document;
    assert_eq!(merged.resolve().unwrap()[1].fields["xref"], "a");
    let renamed = merged
        .apply_patch(&[refkit_core::BibEdit::RenameEntry {
            entry_id: merged.entry_occurrences()[0].id,
            key: "new".into(),
        }])
        .unwrap()
        .document;
    assert_eq!(renamed.resolve().unwrap()[1].fields["xref"], "new");
    let options = refkit_core::TidyOptions {
        generate_keys: Some("prefix[fulltitle:lower]".into()),
        ..Default::default()
    };
    let tidied = refkit_core::tidy_bibtex(&merged.render().unwrap(), options).unwrap();
    let resolved = RawDocument::parse(&tidied.bibtex).resolve().unwrap();
    assert!(
        resolved
            .iter()
            .any(|entry| entry.key == "prefixc" && entry.fields["xref"] == "prefixa")
    );
}
