from __future__ import annotations

from pathlib import Path

import refkit as rk

FIXTURES = Path(__file__).parent / "fixtures"


def rendered_tree_kinds(rendered: rk.Rendered) -> list[str]:
    return [node["kind"] for node in rendered.tree]


def raw_block_starts(raw: rk.BibDocument) -> list[int]:
    return [block["span"][0] for block in raw.blocks]


def failed_block_errors(raw: rk.BibDocument) -> list[str]:
    return [block["error"] for block in raw.failed_blocks]


def tidy_warning_codes(result: rk.TidyResult) -> list[str]:
    return [warning.code for warning in result.warnings]


def duplicate_entry_titles(raw: rk.BibDocument, key: str) -> list[str]:
    return [
        field.value for entry in raw.entries.get_all(key) for field in entry.fields.get_all("title")
    ]


def test_type_checked_structured_return_samples() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")
    raw = rk.BibDocument.read(FIXTURES / "raw.bib")

    rendered = doc.render([rk.Citation("first", "doe2024")])

    assert rendered_tree_kinds(rendered["first"])
    assert all(start >= 0 for start in raw_block_starts(raw))
    assert all(error for error in failed_block_errors(raw))
    duplicate_raw = rk.BibDocument.read(FIXTURES / "raw-duplicates.bib")
    assert duplicate_entry_titles(duplicate_raw, "dup") == [
        "First Title",
        "Second Title",
        "Duplicate Entry",
    ]
    tidied = rk.BibDocument.parse("@article{typed, title={Typed Contract}, year={2024}}\n").tidy(
        options=rk.TidyOptions(strip_comments=True)
    )
    assert isinstance(tidied.bibtex, str)
    assert tidy_warning_codes(tidied) == []
