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


def test_type_checked_structured_return_samples() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")
    raw = rk.BibDocument.read(FIXTURES / "raw.bib")

    assert rendered_tree_kinds(doc.cite("doe2024"))
    assert raw_block_starts(raw)[0] == 0
    assert failed_block_errors(raw) == ["entry ended before closing delimiter"]
