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
    return {
        "key": value.key,
        "entry_type": value.entry_type,
        "title": value.title,
        "date": value.date,
        "doi": value.doi,
        "volume": value.volume,
        "parents": [entry(parent) for parent in value.parents],
    }


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
            ["key", "entry_type", "type", "title", "date", "doi", "volume"], keys=selected
        ),
        "selected_records": [entry(value) for value in library.get_many(selected)],
        "selected_by_type": [
            entry(value) for value in library.select(case.get("selector", "article | book"))
        ],
        "missing": library.get("absent-reference"),
    }


def render_case(case: dict[str, Any]) -> dict[str, Any]:
    library = rk.Library.parse_bibtex(case["source"])
    style = rk.Style.from_xml(case["xml"]) if "xml" in case else rk.Style.load(case["style"])
    document = rk.Document(library, style, locale="en-US")
    citations = [
        rk.Citation(
            value["id"],
            rk.CitationGroup(
                rk.Cite(item["key"], locator=item.get("locator"), label=item.get("label"))
                for item in value["items"]
            ),
            note_number=value.get("note_number"),
        )
        for value in case["citations"]
    ]
    return {
        "rendered": rendered_document(document.render(citations)),
        "cited_bibliography": rendered(document.cited_bibliography(citations)),
        "full_bibliography": rendered(document.full_bibliography()),
        "fresh_render": rendered_document(document.render(citations[-1:])),
        "repeated_render": rendered_document(document.render(citations)),
    }


def raw_state(document: rk.BibDocument) -> dict[str, Any]:
    return {
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
        field.value = edit["value"]
    return {"before": before, "after": raw_state(document)}


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
        [
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
    runners = {"library": library_case, "render": render_case, "raw": raw_case, "tidy": tidy_case}
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
