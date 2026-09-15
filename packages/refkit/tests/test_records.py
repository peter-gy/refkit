from __future__ import annotations

import json
from typing import Any, cast

import pytest

import refkit as rk


def test_complete_records_are_detached_and_reconstruct_rendering() -> None:
    records: list[rk.Entry] = [
        {
            "key": "council",
            "entry_type": "Book",
            "title": {"chunks": [{"kind": "normal", "text": "Annual Report"}]},
            "authors": [{"kind": "organization", "name": "Research Council"}],
            "date": {"value": {"kind": "point", "date": {"year": 2024}}},
            "extensions": {"application": {"reviewed": True}},
        }
    ]
    library = rk.Library.from_records(records)
    snapshot = library.to_json()
    restored = rk.Library.from_json(snapshot)
    assert restored.to_records() == library.to_records()
    assert restored["council"]["authors"] == records[0]["authors"]
    rendered = rk.Document(restored, rk.Style.load("apa")).render([rk.Citation("intro", "council")])
    assert rendered["intro"].text == "(Research Council, 2024)"
    detached = library.to_records()
    title = detached[0]["title"]
    assert title is not None
    title["chunks"][0]["text"] = "Changed"
    assert library.to_json() == snapshot
    assert rk.Library.from_records(restored.to_records()).to_json() == snapshot


def test_structured_input_rejects_conflicts_and_preserves_date_ranges() -> None:
    with pytest.raises(ValueError, match="duplicate entry key"):
        rk.Library.from_records([{"key": "a", "entry_type": "Book"}] * 2)
    with pytest.raises(ValueError, match="unknown field"):
        rk.Library.from_records(cast(Any, [{"key": "a", "entry_type": "Book", "typo": True}]))
    with pytest.raises(TypeError):
        rk.Library.from_records(cast(Any, "text"))
    library = rk.Library.parse_bibtex(
        "@book{a,title={A {Protected} Title},date={2020/2024?},custom={Kept}}"
    )
    record = library["a"]
    date = record["date"]
    assert date is not None and date["uncertain"]
    assert date["value"]["kind"] == "range"
    assert "custom" in record["extensions"]["biblatex"]
    assert json.loads(library.to_json())["schema_version"] == 1
