from __future__ import annotations

from pathlib import Path

import refkit as rk
from refkit.types import Diagnostic, RawBlock, RenderedTree, ResolvedBibEntry, TidyRename

FIXTURES = Path(__file__).parent / "fixtures"


def rendered_tree_kinds(tree: RenderedTree) -> list[str]:
    return [node["kind"] for node in tree]


def raw_block_starts(blocks: list[RawBlock]) -> list[int]:
    return [block["span"][0] for block in blocks]


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

    assert rendered_tree_kinds(rendered["first"].tree) == ["Text", "Element", "Text"]
    assert raw_block_starts(raw.blocks)[0] == 0
    assert failed_block_errors(raw) == ["entry ended before closing delimiter"]
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


def diagnostic_messages(diagnostics: list[Diagnostic]) -> list[str]:
    return [diagnostic["message"] for diagnostic in diagnostics]


def renamed_keys(renames: list[TidyRename]) -> list[str]:
    return [rename["new_key"] for rename in renames]


def resolved_fields(entries: list[ResolvedBibEntry]) -> list[dict[str, str]]:
    return [entry["fields"] for entry in entries]


def test_public_record_types_support_consumer_annotations() -> None:
    source = "@article{old,author={Doe, Jane},title={Work},year={2024}}"
    library = rk.Library.parse_bibtex(source)
    result = rk.tidy_bibtex(source, options=rk.TidyOptions(generate_keys="[auth:lower][year]"))
    assert diagnostic_messages(library.diagnostics) == []
    assert renamed_keys(result.renames) == ["doe2024"]
    assert resolved_fields(rk.BibDocument.parse(source).resolve()) == [
        {"author": "Doe, Jane", "title": "Work", "year": "2024"}
    ]
