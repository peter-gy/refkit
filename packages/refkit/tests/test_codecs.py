from __future__ import annotations

import json

import pytest

import refkit as rk
from refkit.types import BibliographyFormat, DecodeReport, EncodeReport

BOOK = (
    '[{"id":"book","type":"book","title":"A Book",'
    '"author":[{"family":"Doe","given":"Jane"}],'
    '"issued":{"date-parts":[[2024]]},"publisher":"Press"}]'
)


@pytest.mark.parametrize("target", ["biblatex", "hayagriva", "csl-json"])
def test_codec_reports_construct_renderable_libraries(target: BibliographyFormat) -> None:
    decoded: DecodeReport = rk.decode(BOOK, format="csl-json", loss="error")
    encoded: EncodeReport = rk.encode(decoded["library"], format=target, loss="error")
    assert encoded["format"] == target
    assert not any(issue["lossy"] for issue in encoded["issues"])
    restored = rk.decode(encoded["text"], format=target, loss="error")["library"]
    result = rk.Document(restored, rk.Style.load("apa")).render([rk.Citation("intro", "book")])
    assert result["intro"].text == "(Doe, 2024)"


def test_loss_refusal_preserves_records_and_exposes_issues() -> None:
    source = '[{"id":"a","type":"book","issued":{"date-parts":[[2020],[2024]]}}]'
    library = rk.decode(source, format="csl-json")["library"]
    before = library.to_json()
    with pytest.raises(rk.ConversionError) as failure:
        rk.encode(library, format="hayagriva", loss="error")
    assert any(
        issue["path"].startswith("date") and issue["lossy"] for issue in failure.value.issues
    )
    assert failure.value.diagnostics == []
    assert library.to_json() == before
    reported = rk.encode(library, format="hayagriva", loss="report")
    assert any(issue["lossy"] for issue in reported["issues"])


def test_conversion_retains_parser_diagnostics_and_source_identity() -> None:
    report = rk.convert(
        "@book{a,title=missing}",
        source_format="biblatex",
        target_format="hayagriva",
        recovery="report",
    )
    assert report["source_format"] == "biblatex"
    assert report["target_format"] == "hayagriva"
    assert report["diagnostics"][0]["code"] == "unknown_abbreviation"
    assert any(issue["stage"] == "decode" and issue["lossy"] for issue in report["issues"])
    with pytest.raises(rk.ConversionError) as failure:
        rk.decode("@book{a,title=missing}", format="biblatex", recovery="report", loss="error")
    assert failure.value.diagnostics[0]["span"] is not None
    with pytest.raises(rk.ConversionError) as malformed:
        rk.decode("[", format="csl-json")
    assert malformed.value.issues[0]["code"] == "invalid_source"


def test_csl_custom_data_and_numeric_ids_roundtrip() -> None:
    source = '[{"id":1.0,"type":"book","custom":{"rating":3},"accessed":{"date-parts":[[2024]]}}]'
    decoded = rk.decode(source, format="csl-json", loss="error")
    assert decoded["library"].keys() == ["1"]
    encoded = rk.encode(decoded["library"], format="csl-json", loss="error")
    assert json.loads(encoded["text"])[0]["custom"] == {"rating": 3}
    assert "accessed" in json.loads(encoded["text"])[0]
