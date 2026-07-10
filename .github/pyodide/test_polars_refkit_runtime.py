from __future__ import annotations

import polars as pl

import polars_refkit as prk

BIBTEX = """
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  year = {2024}
}
"""


def test_polars_parse_render_and_tidy_contracts() -> None:
    frame = pl.DataFrame(
        {
            "bibtex": [BIBTEX],
            "key": ["doe2024"],
        }
    )

    row = frame.select(
        report=prk.parse_report("bibtex"),
        citation=prk.cite("bibtex", "key", style="apa"),
        tidy=prk.tidy_bibtex_report("bibtex", sort_fields=True),
    ).to_dicts()[0]

    assert prk.__version__
    assert row["report"]["ok"] is True
    assert row["report"]["entry_count"] == 1
    assert row["citation"] == "(Doe, 2024)"
    assert row["tidy"]["ok"] is True
    assert row["tidy"]["count"] == 1


def test_polars_row_failures_stay_local() -> None:
    frame = pl.DataFrame({"bibtex": [BIBTEX, "@broken{missing"]})

    rows = frame.select(
        count=prk.entry_count("bibtex"),
        report=prk.parse_report("bibtex"),
    ).to_dicts()

    assert rows[0]["count"] == 1
    assert rows[0]["report"]["ok"] is True
    assert rows[1]["count"] is None
    assert rows[1]["report"]["ok"] is False
