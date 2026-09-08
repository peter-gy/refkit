from __future__ import annotations

from dataclasses import replace

import pytest

from refkit_bench.fixtures import (
    Record,
    bibtex_for_records,
    csl_json_for_records,
    load_workload,
)
from refkit_bench.rendering import prepare_render


@pytest.mark.parametrize("package", ["refkit", "citeproc-py"])
@pytest.mark.parametrize("lane", ["render.citation", "render.bibliography", "render.document"])
def test_real_bibliography_matches_complete_ordered_contract(package: str, lane: str) -> None:
    prepared = prepare_render(lane, load_workload("real"), package)
    first = prepared.operation()
    prepared.check(first)
    second = prepared.operation()
    prepared.check(second)
    assert second == first


@pytest.mark.parametrize("package", ["refkit", "citeproc-py"])
def test_document_retains_authors_optional_fields_and_citation_order(package: str) -> None:
    records = (
        Record(
            key="z",
            family="Zulu",
            given="Zoë",
            title="Zebra",
            year=2024,
            volume=2,
            page_start=None,
            page_end=None,
            doi="10.1234/ABC",
            pages="6:1-6:13",
            container="Journal",
            authors=(("Zulu", "Zoë"), ("Adams", "Amy")),
        ),
        Record(
            key="a",
            family="Acme",
            given="",
            title="Antelope",
            year=2023,
            volume=None,
            page_start=None,
            page_end=None,
            doi=None,
            container="",
        ),
    )
    workload = replace(
        load_workload("tiny"),
        records=records,
        bibtex=bibtex_for_records(records),
        csl_json=csl_json_for_records(records),
    )
    prepared = prepare_render("render.document", workload, package)
    result = prepared.operation()
    assert result == {
        "citations": ["(Zulu / Adams, 2024)", "(Acme, 2023)"],
        "bibliography": [
            "Acme | 2023 | Antelope",
            "Zulu, Zoë / Adams, Amy | 2024 | Zebra | Journal | 2 | 6:1–6:13 | 10.1234/ABC",
        ],
    }
    prepared.check(result)


@pytest.mark.parametrize("mutation", ["author", "punctuation", "order", "duplicate"])
def test_rendering_check_rejects_changed_content_and_order(mutation: str) -> None:
    prepared = prepare_render("render.bibliography", load_workload("real"), "refkit")
    rows = prepared.operation()
    assert isinstance(rows, list)
    changed = rows.copy()
    if mutation == "author":
        changed[0] = changed[0].replace(" / Xia, Bowei", "")
    elif mutation == "punctuation":
        changed[0] = changed[0].replace(" | ", " ")
    elif mutation == "order":
        changed.reverse()
    else:
        changed[1] = changed[0]
    with pytest.raises(AssertionError, match="output mismatch"):
        prepared.check(changed)
