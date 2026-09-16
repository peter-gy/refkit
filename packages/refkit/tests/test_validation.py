from __future__ import annotations

import pytest

import refkit as rk
from refkit.types import ValidationReport


def test_record_validation_reports_detached_inspect_only_findings() -> None:
    library = rk.Library.from_records(
        [
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
        ]
    )
    before = library.to_json()
    report: ValidationReport = library.validate()
    assert report["profile"] == "records"
    assert not report["valid"]
    invalid = next(issue for issue in report["issues"] if issue["code"] == "invalid_identifier")
    assert invalid["target"] == {
        "entry": "b",
        "path": "identifiers.isbn",
        "entry_id": None,
        "field_id": None,
        "span": None,
    }
    shared = next(issue for issue in report["issues"] if issue["code"] == "shared_identifier")
    assert shared["related"][0]["entry"] == "b"
    report["issues"].clear()
    assert library.validate()["issues"]
    assert library.to_json() == before


def test_biblatex_validation_has_byte_spans_and_separate_parse_failures() -> None:
    source = "% é\n@article{a,title={A},doi={bad},crossref={missing}}"
    document = rk.BibDocument.parse(source)
    report: ValidationReport = document.validate()
    assert report["profile"] == "biblatex"
    invalid = next(issue for issue in report["issues"] if issue["code"] == "invalid_identifier")
    span = invalid["target"]["span"]
    assert isinstance(span, tuple)
    assert b"bad" in source.encode()[span[0] : span[1]]
    assert invalid["target"]["entry_id"] == 0
    assert invalid["target"]["field_id"] == 1
    assert any(issue["code"] == "unresolved_reference" for issue in report["issues"])
    assert document.to_bibtex() == source
    with pytest.raises(rk.ParseError):
        rk.BibDocument.parse("@book{a,title=undefined}").validate()
