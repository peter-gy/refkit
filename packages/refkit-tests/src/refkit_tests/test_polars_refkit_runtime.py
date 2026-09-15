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
        tidy=prk.tidy_bibtex_report("bibtex", options={"sort_fields": True}),
    ).to_dicts()[0]

    assert prk.__version__
    assert row["report"]["ok"] is True
    assert row["report"]["entry_count"] == 1
    assert row["citation"] == "(Doe, 2024)"
    assert row["tidy"]["ok"] is True
    assert row["tidy"]["count"] == 1


def test_polars_row_failures_stay_local() -> None:
    frame = pl.DataFrame(
        {"bibtex": [BIBTEX, "@broken{missing", "@book{loop,title={Loop},crossref={loop}}"]}
    )

    rows = frame.select(
        count=prk.entry_count("bibtex", recovery="error"),
        report=prk.parse_report("bibtex", recovery="error"),
    ).to_dicts()

    assert rows[0]["count"] == 1
    assert rows[0]["report"]["ok"] is True
    assert rows[1]["count"] is None
    assert rows[1]["report"]["ok"] is False
    assert rows[2]["count"] is None
    assert rows[2]["report"]["ok"] is False
    assert rows[2]["report"]["diagnostics"][0]["code"] == "cyclic_reference"


def test_recovered_literal_render_values_match_reports() -> None:
    title = "Recovery Café " * 4096
    source = pl.lit(BIBTEX + f"@book{{bad,title={{{title}}},year={{nonsense}}}}")
    frame = pl.DataFrame(
        {
            "key": ["doe2024", "missing", None, "doe2024", "doe2024"],
            "keys": [["doe2024"], ["missing"], None, [], [None]],
        },
        schema={"key": pl.String, "keys": pl.List(pl.String)},
    )
    expressions = {
        "single": prk.cite(source, "key", recovery="report", output="rendered"),
        "text": prk.cite_each(source, "keys", recovery="report"),
        "html": prk.cite_each(source, "keys", recovery="report", output="html"),
        "rendered": prk.cite_each(source, "keys", recovery="report", output="rendered"),
        "report": prk.render_report(source, "keys", recovery="report"),
        "group": prk.cite_group(source, "keys", recovery="report", output="rendered"),
        "group_report": prk.render_report(source, "keys", grouped=True, recovery="report"),
    }
    eager = frame.select(**expressions)
    assert eager.equals(frame.lazy().select(**expressions).collect())
    rows = eager.to_dicts()
    assert rows[0]["single"] == {"text": "(Doe, 2024)", "html": "(Doe, 2024)"}
    assert rows[1]["single"] is None
    assert rows[2]["single"] is None
    assert rows[0]["report"]["diagnostics"][0]["entry"] == "bad"
    assert rows[1]["report"]["error_code"] == "missing_key"
    assert rows[3]["rendered"] == []
    assert rows[3]["group_report"]["error_code"] == "render_error"
    assert rows[4]["report"] is None
    for row in rows:
        report = row["report"]
        if report is not None and report["ok"]:
            assert row["rendered"] == report["citations"]
            assert row["text"] == [citation["text"] for citation in report["citations"]]
            assert row["html"] == [citation["html"] for citation in report["citations"]]
        else:
            assert row["rendered"] is None
            assert row["text"] is None
            assert row["html"] is None
        group = row["group_report"]
        expected_group = group["citations"][0] if group is not None and group["ok"] else None
        assert row["group"] == expected_group
