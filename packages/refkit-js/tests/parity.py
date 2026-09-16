"""Compare installed Python and JavaScript adapters on shared bibliography inputs."""

from __future__ import annotations

import difflib
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

import refkit as rk

ROOT = Path(__file__).resolve().parents[3]
FIXTURES = ROOT / "packages/refkit/tests/fixtures"


def fixture(name: str) -> str:
    return (FIXTURES / name).read_text(encoding="utf-8")


def entry(value: rk.Entry) -> dict[str, Any]:
    return dict(value)


def rendered(value: rk.Rendered) -> dict[str, Any]:
    return {"text": value.text, "html": value.html, "tree": value.tree, "layout": value.layout}


def rendered_document(value: rk.RenderedDocument) -> dict[str, Any]:
    return {
        "citation_order": value.citation_order,
        "citations": {key: rendered(value[key]) for key in value.citation_order},
        "bibliography": rendered(value.bibliography),
    }


def library_case(case: dict[str, Any]) -> dict[str, Any]:
    try:
        library = (
            rk.Library.parse_yaml(case["source"])
            if case["format"] == "yaml"
            else rk.Library.parse_bibtex(case["source"], recovery=case.get("recovery", "error"))
        )
    except rk.ParseError as error:
        if case.get("expect_error") != "ParseError":
            raise
        return {"error": "ParseError", "diagnostics": error.diagnostics}
    if "expect_error" in case:
        raise AssertionError(f"{case['name']} must reject the source")
    keys = library.keys()
    selected = [keys[-1], keys[0], keys[-1]] if keys else []
    return {
        "keys": keys,
        "records": [entry(value) for value in library.values()],
        "diagnostics": library.diagnostics,
        "projection": library.project(),
        "selected_projection": library.project(
            ["key", "entry_type", "title", "date", "doi", "volume"], keys=selected
        ),
        "selected_records": [entry(value) for value in library.get_many(selected)],
        "selected_by_type": [
            entry(value) for value in library.select(case.get("selector", "article | book"))
        ],
        "missing": library.get("absent-reference"),
    }


def records_case(case: dict[str, Any]) -> dict[str, Any]:
    library = rk.Library.from_records(case["records"])
    restored = rk.Library.from_json(library.to_json())
    reconstructed = rk.Library.from_records(restored.to_records())
    return {
        "records": reconstructed.to_records(),
        "snapshot": reconstructed.to_json(),
        "projection": reconstructed.project(),
        "rendered": rendered(rk.Document(reconstructed, rk.Style.load("apa")).full_bibliography()),
    }


def validation_case(case: dict[str, Any]) -> dict[str, Any]:
    try:
        if "records" in case:
            return dict(rk.Library.from_records(case["records"]).validate())
        return dict(rk.BibDocument.parse(case["source"]).validate())
    except rk.ParseError as error:
        return {"error": "ParseError", "diagnostics": error.diagnostics}


def codec_case(case: dict[str, Any]) -> dict[str, Any]:
    try:
        report = rk.convert(
            case["source"],
            source_format=case["source_format"],
            target_format=case["target_format"],
            loss=case.get("loss", "report"),
            recovery=case.get("recovery", "error"),
        )
        restored = rk.decode(report["text"], format=case["target_format"])
        return {
            "report": report,
            "records": restored["library"].to_records(),
            "issues": restored["issues"],
        }
    except rk.ConversionError as error:
        return {
            "error": "ConversionError",
            "issues": error.issues,
            "diagnostics": error.diagnostics,
        }


def render_case(case: dict[str, Any]) -> dict[str, Any]:
    library = rk.Library.parse_bibtex(case["source"])
    style = (
        rk.Style.from_xml(case["xml"], parent_xml=case.get("parent_xml"))
        if "xml" in case
        else rk.Style.load(case["style"])
    )
    document = rk.Document(library, style, locale="en-US")
    citations = [
        rk.Citation(
            value["id"],
            rk.CitationGroup(
                rk.Cite(
                    item["key"],
                    locator=item.get("locator"),
                    label=item.get("label"),
                    purpose=item.get("purpose", "normal"),
                )
                for item in value["items"]
            ),
            note_number=value.get("note_number"),
        )
        for value in case["citations"]
    ]
    return {
        "style_id": style.csl_id,
        "style_title": style.title,
        "rendered": rendered_document(document.render(citations)),
        "cited_bibliography": rendered(document.cited_bibliography(citations)),
        "full_bibliography": rendered(document.full_bibliography()),
        "fresh_render": rendered_document(document.render(citations[-1:])),
        "repeated_render": rendered_document(document.render(citations)),
    }


def raw_state(document: rk.BibDocument) -> dict[str, Any]:
    try:
        resolution: Any = document.resolve()
    except rk.ParseError as error:
        resolution = {"error": "ParseError", "diagnostics": error.diagnostics}
    return {
        "resolution": resolution,
        "bibtex": document.to_bibtex(),
        "diagnostics": document.diagnostics,
        "comments": document.comments,
        "preamble": document.preamble,
        "strings": document.strings,
        "failed_blocks": document.failed_blocks,
        "blocks": document.blocks,
        "keys": document.entries.unique_keys(),
        "occurrence_keys": document.entries.occurrence_keys(),
        "entries": [
            {
                "key": value.key,
                "kind": value.kind,
                "span": value.span,
                "field_keys": value.fields.unique_keys(),
                "field_occurrence_keys": value.fields.occurrence_keys(),
                "fields": [
                    {"name": field.name, "value": field.value, "span": field.span}
                    for field in value.fields.occurrences()
                ],
            }
            for value in document.entries.occurrences()
        ],
    }


def raw_case(case: dict[str, Any]) -> dict[str, Any]:
    document = rk.BibDocument.parse(case["source"])
    before = raw_state(document)
    for edit in case["edits"]:
        value = document.entries.get_all(edit["key"])[edit["entry"]]
        field = value.fields.get_all(edit["field"])[edit["occurrence"]]
        document = document.apply_patch(
            [
                {
                    "kind": "set_field",
                    "entry_id": field.entry_id,
                    "field_id": field.id,
                    "value": edit["value"],
                }
            ]
        )["document"]
    return {"before": before, "after": raw_state(document)}


def patch_case(case: dict[str, Any]) -> dict[str, Any]:
    document = rk.BibDocument.parse(case["source"])
    before = raw_state(document)
    try:
        result = document.apply_patch(case["patch"])
        return {
            "before": before,
            "after": raw_state(result["document"]),
            "original_after": raw_state(document),
            "report": {
                "changes": result["changes"],
                "entries": result["entries"],
                "warnings": result["warnings"],
            },
        }
    except rk.PatchError as error:
        return {
            "error": "PatchError",
            "code": error.code,
            "operation": error.operation,
            "original_after": raw_state(document),
        }


def merge_case(case: dict[str, Any]) -> dict[str, Any]:
    document = rk.BibDocument.parse(case["source"])
    report = document.find_duplicates(rules=case.get("rules"))
    try:
        plan = document.plan_merge(
            case["entries"],
            retain=case["retain"],
            fields=case.get("fields"),
            entry_type=case.get("entry_type"),
        )
        updated = (
            document.apply_patch(plan["patch"])["document"] if plan["patch"] is not None else None
        )
        return {
            "report": report,
            "plan": plan,
            "source": updated.to_bibtex() if updated else None,
            "original": document.to_bibtex(),
        }
    except rk.MergeError as error:
        return {
            "report": report,
            "error": "MergeError",
            "code": error.code,
            "original": document.to_bibtex(),
        }


def tidy_case(case: dict[str, Any]) -> dict[str, Any]:
    result = rk.tidy_bibtex(case["source"], options=rk.TidyOptions(**case["options"]))
    return {
        "bibtex": result.bibtex,
        "count": result.count,
        "warnings": [
            {"code": warning.code, "rule": warning.rule, "message": warning.message}
            for warning in result.warnings
        ],
        "renames": result.renames,
    }


def cases() -> list[dict[str, Any]]:
    bibliography = fixture("basic.bib")
    recovery = (
        "@string{café={Café}}\n"
        "@book{good,title=café,year={2024}}\n"
        "@book{unknown,title=未定,year=bogus}\n"
        "@broken{missing, title={Unclosed}\n"
    )
    tidy_source = r"""% bibliography comment
@ARTICLE{zOldKey,
  TITLE = {{{A VERY LONG TITLE About References & Reliable Bibliography Formatting}}},
  AUTHOR = {DOE, JANE and Roe, Richard},
  YEAR = {2024},
  MONTH = {January},
  DOI = {10.1234/parity},
  URL = {https://example.com/a b},
  ABSTRACT = {{Protected abstract}},
  NOTE = {Working note},
  KEYWORDS = {},
  PAGES = {1--20},
  PAGES = {1--20}
}
@article{aSecondKey,
  title={A VERY LONG TITLE About References & Reliable Bibliography Formatting},
  author={DOE, JANE and Roe, Richard},
  year={2024},
  doi={10.1234/parity},
  volume={7}
}
"""
    values = [
        {"name": "bibtex-records", "kind": "library", "format": "bibtex", "source": bibliography},
        {
            "name": "biblatex-records",
            "kind": "library",
            "format": "bibtex",
            "source": fixture("typst-biblatex.bib"),
            "recovery": "report",
        },
        {
            "name": "yaml-records",
            "kind": "library",
            "format": "yaml",
            "source": fixture("hayagriva-rich.yaml"),
        },
        {
            "name": "yaml-parents",
            "kind": "library",
            "format": "yaml",
            "source": fixture("parent.yaml"),
            "selector": "article > periodical[volume]",
        },
        {
            "name": "unicode-recovery",
            "kind": "library",
            "format": "bibtex",
            "source": recovery,
            "recovery": "report",
        },
        {
            "name": "unicode-parse-error",
            "kind": "library",
            "format": "bibtex",
            "source": recovery,
            "expect_error": "ParseError",
        },
    ]
    citations = [
        {
            "id": "__proto__",
            "items": [{"key": "roe2022"}, {"key": "doe2024", "locator": "12", "label": "page"}],
            "note_number": 1,
        },
        {
            "id": "constructor",
            "items": [{"key": "doe2024", "locator": "3", "label": "chapter"}],
            "note_number": 2,
        },
        {"id": "repeat", "items": [{"key": "roe2022"}], "note_number": 4},
    ]
    values.extend(
        {
            "name": f"render-{style}",
            "kind": "render",
            "source": bibliography,
            "style": style,
            "citations": citations,
        }
        for style in ["apa", "ieee"]
    )
    values.append(
        {
            "name": "render-notes",
            "kind": "render",
            "source": bibliography,
            "xml": """<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="note">
  <info><title>Note context</title><id>https://example.test/notes</id>
    <updated>2024-01-01T00:00:00Z</updated></info>
  <citation><layout delimiter=", "><number variable="first-reference-note-number"/>
    <choose><if position="near-note"><text value=" near"/></if>
      <else><text value=" far"/></else></choose></layout></citation>
  <bibliography hanging-indent="true" line-spacing="2" entry-spacing="3">
    <layout><text variable="title" font-style="italic"/></layout></bibliography>
</style>""",
            "citations": [
                {"id": "first", "items": [{"key": "doe2024"}], "note_number": 8},
                {"id": "later", "items": [{"key": "doe2024"}], "note_number": 9},
            ],
        }
    )
    values.extend(
        {
            "name": f"purpose-{style}-{purpose}",
            "kind": "render",
            "source": bibliography,
            "style": style,
            "citations": [
                {"id": "first", "items": [{"key": "doe2024", "purpose": purpose}]},
                {
                    "id": "group",
                    "items": [
                        {"key": "roe2022", "purpose": purpose},
                        {"key": "doe2024", "purpose": purpose, "locator": "12"},
                    ],
                },
                {"id": "repeat", "items": [{"key": "doe2024", "purpose": purpose}]},
            ],
        }
        for style in ["apa", "ieee"]
        for purpose in ["normal", "author", "year", "full", "prose"]
    )
    values.append(
        {
            "name": "dependent-style",
            "kind": "render",
            "source": bibliography,
            "xml": (
                '<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" '
                'default-locale="de-DE"><info><title>Child</title>'
                '<id>https://example.com/child</id><link rel="independent-parent" '
                'href="https://example.com/parent"/></info></style>'
            ),
            "parent_xml": (
                '<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">'
                "<info><title>Parent</title><id>https://example.com/parent</id></info>"
                '<citation><layout><text variable="title"/></layout></citation>'
                '<bibliography><layout><text variable="title"/></layout></bibliography></style>'
            ),
            "citations": citations,
        }
    )
    values.extend(
        [
            {
                "name": "raw-resolved-fields",
                "kind": "raw",
                "source": (
                    '@string{host="https://example.test/"}\n'
                    "@misc{entry,title={A {Protected} Title},url=host # {paper},custom={Kept}}"
                ),
                "edits": [
                    {
                        "key": "entry",
                        "entry": 0,
                        "field": "url",
                        "occurrence": 0,
                        "value": "https://example.test/revised",
                    },
                ],
            },
            {
                "name": "raw-duplicate-edits",
                "kind": "raw",
                "source": fixture("raw-duplicates.bib"),
                "edits": [
                    {
                        "key": "dup",
                        "entry": 0,
                        "field": "TITLE",
                        "occurrence": 1,
                        "value": "Updated café",
                    },
                    {
                        "key": "dup",
                        "entry": 1,
                        "field": "title",
                        "occurrence": 0,
                        "value": "Second occurrence",
                    },
                ],
            },
            {
                "name": "raw-block-preservation",
                "kind": "raw",
                "source": fixture("raw.bib"),
                "edits": [
                    {
                        "key": "doe2024",
                        "entry": 0,
                        "field": "title",
                        "occurrence": 0,
                        "value": "Source spans remain stable",
                    },
                ],
            },
        ]
    )
    tidy_options = {
        "tidy-defaults": {},
        "tidy-layout": {
            "curly": True,
            "numeric": True,
            "months": True,
            "tab": True,
            "tidy_comments": False,
            "align": 18,
            "blank_lines": True,
            "sort": True,
            "sort_fields": ["author", "title", "year"],
            "trailing_commas": True,
            "wrap": 48,
        },
        "tidy-text": {
            "omit": ["note"],
            "strip_enclosing_braces": True,
            "drop_all_caps": True,
            "escape": True,
            "strip_comments": True,
            "encode_urls": True,
            "remove_empty_fields": True,
            "remove_duplicate_fields": True,
            "lowercase": False,
            "enclosing_braces": ["title"],
            "remove_braces": ["abstract"],
        },
        "tidy-duplicate-keys": {
            "space": 4,
            "duplicates": ["doi", "key"],
            "merge": "combine",
            "generate_keys": "[auth:lower][year]",
            "max_authors": 1,
        },
    }
    values.extend(
        {"name": name, "kind": "tidy", "source": tidy_source, "options": options}
        for name, options in tidy_options.items()
    )
    return values


def main() -> None:
    inputs = cases()
    merge_source = (
        "@string{press={Press}}@book{a,title={First},doi={10.1234/work}}"
        "@book{b,title={Second},doi={10.1234/work},publisher=press,"
        "url={https://example.org/a%2Fb}}@misc{child,crossref={b}}"
    )
    inputs.extend(
        [
            {
                "name": "merge-unresolved",
                "kind": "merge",
                "source": merge_source,
                "entries": [0, 1],
                "retain": 0,
            },
            {
                "name": "merge-accepted",
                "kind": "merge",
                "source": merge_source,
                "entries": [0, 1],
                "retain": 0,
                "fields": [{"kind": "take", "name": "title", "entry_id": 1, "field_id": 0}],
            },
            {
                "name": "merge-invalid",
                "kind": "merge",
                "source": merge_source,
                "entries": [0, 0],
                "retain": 0,
            },
            {
                "name": "merge-cycle",
                "kind": "merge",
                "source": "@book{a,title={A},crossref={b}}@book{b,title={A}}",
                "entries": [0, 1],
                "retain": 0,
            },
            {
                "name": "patch-expression",
                "kind": "patch",
                "source": "@string{press={Press}}@book{a,title={A}}",
                "patch": [
                    {
                        "kind": "add_field",
                        "entry_id": 0,
                        "name": "publisher",
                        "value": "press # { Extra}",
                        "expression": True,
                    }
                ],
            },
        ]
    )
    inputs.extend(
        [
            {
                "name": "patch-structural",
                "kind": "patch",
                "source": "% é\n@Book(a,title={Old},note={Remove})\n@misc{b,title={B}}",
                "patch": [
                    {"kind": "set_field", "entry_id": 0, "field_id": 0, "value": "Updated"},
                    {"kind": "remove_field", "entry_id": 0, "field_id": 1},
                    {"kind": "add_field", "entry_id": 0, "name": "doi", "value": "10.1234/a"},
                    {"kind": "rename_entry", "entry_id": 0, "key": "renamed"},
                    {"kind": "set_entry_type", "entry_id": 0, "entry_type": "article"},
                    {"kind": "remove_entry", "entry_id": 1},
                    {
                        "kind": "add_entry",
                        "key": "new",
                        "entry_type": "book",
                        "before": 0,
                        "fields": [{"name": "title", "value": "New"}],
                    },
                ],
            },
            {
                "name": "patch-reference",
                "kind": "patch",
                "source": (
                    "@string{parent={a}}@book{a,title={A}}@misc{child,crossref=parent,xdata={a}}"
                ),
                "patch": [{"kind": "rename_entry", "entry_id": 0, "key": "New_Key"}],
            },
            {
                "name": "patch-overlap",
                "kind": "patch",
                "source": "@book{a,title={A}}",
                "patch": [
                    {"kind": "remove_entry", "entry_id": 0},
                    {"kind": "set_field", "entry_id": 0, "field_id": 0, "value": "Updated"},
                ],
            },
            {
                "name": "patch-ambiguous",
                "kind": "patch",
                "source": "@book{a,title={A}}@book{a,title={B}}@misc{c,crossref={a}}",
                "patch": [{"kind": "rename_entry", "entry_id": 0, "key": "new"}],
            },
            {
                "name": "patch-invalid-target",
                "kind": "patch",
                "source": "@book{a,title={A}}",
                "patch": [{"kind": "remove_entry", "entry_id": 10}],
            },
            {
                "name": "patch-portable-id-bound",
                "kind": "patch",
                "source": "@book{a,title={A}}",
                "patch": [{"kind": "remove_entry", "entry_id": 4294967296}],
            },
        ]
    )
    inputs.extend(
        [
            {
                "name": "validate-records",
                "kind": "validation",
                "records": [
                    {
                        "key": "a",
                        "entry_type": "Misc",
                        "identifiers": {"doi": "https://doi.org/10.1000/ABC"},
                    },
                    {
                        "key": "b",
                        "entry_type": "Misc",
                        "identifiers": {"doi": "10.1000/abc", "isbn": "bad"},
                    },
                ],
            },
            {
                "name": "validate-biblatex",
                "kind": "validation",
                "source": "% é\n@article{a,title={A},doi={bad},crossref={missing}}",
            },
            {
                "name": "validate-malformed",
                "kind": "validation",
                "source": "@misc{a,title=missing}",
            },
            {
                "name": "validate-unsafe-date",
                "kind": "validation",
                "source": "@misc{a,date={123456X}}",
            },
            {
                "name": "validate-unsafe-month",
                "kind": "validation",
                "source": "@misc{a,year=2024,month=0}",
            },
            {
                "name": "unsafe-date-recovery",
                "kind": "library",
                "format": "bibtex",
                "source": "@misc{bad,date={123456X}} @book{good,title={Good}}",
                "recovery": "report",
            },
        ]
    )
    codec_sources = {
        "biblatex": "@book{a,author={Doe, Jane},title={A Book},year={2024},publisher={Press}}",
        "hayagriva": (
            "a:\n  type: Book\n  author: 'Doe, Jane'\n"
            "  title: A Book\n  date: 2024\n  publisher: Press\n"
        ),
        "csl-json": (
            '[{"id":"a","type":"book","title":"A Book",'
            '"author":[{"family":"Doe","given":"Jane"}],'
            '"issued":{"date-parts":[[2024]]},"publisher":"Press"}]'
        ),
    }
    for source_format, source in codec_sources.items():
        for target_format in codec_sources:
            inputs.append(
                {
                    "name": f"codec-{source_format}-{target_format}",
                    "kind": "codec",
                    "source": source,
                    "source_format": source_format,
                    "target_format": target_format,
                    "loss": "error",
                }
            )
    inputs.extend(
        [
            {
                "name": "codec-range-loss",
                "kind": "codec",
                "source": '[{"id":"a","type":"book","issued":{"date-parts":[[2020],[2024]]}}]',
                "source_format": "csl-json",
                "target_format": "hayagriva",
                "loss": "error",
            },
            {
                "name": "codec-field-loss",
                "kind": "codec",
                "source": "@book{a,title={A {Protected} Title},custom={Value}}",
                "source_format": "biblatex",
                "target_format": "csl-json",
            },
            {
                "name": "codec-invalid",
                "kind": "codec",
                "source": "[",
                "source_format": "csl-json",
                "target_format": "biblatex",
            },
            {
                "name": "codec-recovery",
                "kind": "codec",
                "source": "@book{a,title=unknown}",
                "source_format": "biblatex",
                "target_format": "hayagriva",
                "recovery": "report",
            },
        ]
    )
    inputs.append(
        {
            "name": "structured-records",
            "kind": "records",
            "records": [
                {
                    "key": "org",
                    "entry_type": "Book",
                    "title": {
                        "chunks": [
                            {"kind": "normal", "text": "Study of "},
                            {"kind": "protected", "text": "DNA"},
                            {"kind": "math", "text": "x"},
                        ]
                    },
                    "authors": [{"kind": "organization", "name": "Research Council"}],
                    "date": {
                        "value": {"kind": "range", "start": {"year": 1999}, "end": {"year": 2001}},
                        "uncertain": True,
                    },
                    "identifiers": {"doi": "10.1234/study"},
                    "extensions": {
                        "app": {
                            "entry_type": "keep spelling",
                            "score": 1.25,
                            "nested": [True, None],
                        }
                    },
                }
            ],
        }
    )
    runners = {
        "merge": merge_case,
        "patch": patch_case,
        "validation": validation_case,
        "codec": codec_case,
        "library": library_case,
        "render": render_case,
        "raw": raw_case,
        "tidy": tidy_case,
        "records": records_case,
    }
    expected = {case["name"]: runners[case["kind"]](case) for case in inputs}
    completed = subprocess.run(
        ["node", str(Path(__file__).with_suffix(".mjs"))],
        input=json.dumps(inputs),
        text=True,
        capture_output=True,
        check=False,
        cwd=ROOT,
    )
    if completed.returncode:
        sys.stderr.write(completed.stderr)
        raise SystemExit(completed.returncode)
    actual = json.loads(completed.stdout)
    expected = json.loads(json.dumps(expected))
    mismatches = []
    for name, value in expected.items():
        if value != actual.get(name):
            mismatches.append(name)
            diff = difflib.unified_diff(
                json.dumps(value, indent=2, ensure_ascii=False, sort_keys=True).splitlines(),
                json.dumps(
                    actual.get(name), indent=2, ensure_ascii=False, sort_keys=True
                ).splitlines(),
                fromfile=f"Python: {name}",
                tofile=f"JavaScript: {name}",
                lineterm="",
            )
            sys.stderr.write("\n".join(diff) + "\n")
    if mismatches:
        raise SystemExit(f"Python/JavaScript parity failed: {', '.join(mismatches)}")
    sys.stdout.write(f"Python/JavaScript parity passed: {len(inputs)} scenarios\n")


if __name__ == "__main__":
    main()
