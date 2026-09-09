from __future__ import annotations

import polars as pl
from polars.testing import assert_frame_equal

import polars_refkit as prk


def test_resolve_exposes_fields_in_eager_and_lazy_queries() -> None:
    source = r"""@string{prefix = "https://example.org/"}
@misc{source,
  title = {A {NASA} study},
  url = prefix # "paper",
  custom = {\textit{Evidence}},
  year = 2024
}
@misc{empty}"""
    frame = pl.DataFrame({"bibtex": [source]})
    eager = frame.select(prk.resolve("bibtex"))
    lazy = frame.lazy().select(prk.RefkitExprNamespace(pl.col("bibtex")).resolve())

    assert eager.schema == {
        "resolve": pl.List(
            pl.Struct(
                {
                    "key": pl.String,
                    "entry_type": pl.String,
                    "fields": pl.List(pl.Struct({"name": pl.String, "value": pl.String})),
                }
            )
        )
    }
    assert lazy.collect_schema() == eager.schema
    assert_frame_equal(lazy.collect(), eager)
    resolved = eager["resolve"].to_list()[0]
    assert [(entry["key"], entry["entry_type"]) for entry in resolved] == [
        ("source", "misc"),
        ("empty", "misc"),
    ]
    assert {field["name"]: field["value"] for field in resolved[0]["fields"]} == {
        "title": "A {NASA} study",
        "url": "https://example.org/paper",
        "custom": r"\textit{Evidence}",
        "year": "2024",
    }
    assert resolved[1]["fields"] == []


def test_resolve_keeps_row_failures_and_macro_definitions_isolated() -> None:
    frame = pl.DataFrame(
        {
            "bibtex": [
                '@string{name = "Defined"}\n@misc{valid, title = name}',
                "@misc{missing, title = name}",
                "@misc{broken",
                "",
                None,
            ]
        }
    )

    assert frame.select(prk.resolve("bibtex"))["resolve"].to_list() == [
        [
            {
                "key": "valid",
                "entry_type": "misc",
                "fields": [{"name": "title", "value": "Defined"}],
            }
        ],
        None,
        None,
        [],
        None,
    ]


def test_resolve_literal_broadcasts() -> None:
    expression = prk.resolve(pl.lit("@misc{source, title={Shared}}"))
    result = pl.DataFrame({"row": [1, 2]}).with_columns(expression)
    assert result["resolve"].to_list() == [
        [
            {
                "key": "source",
                "entry_type": "misc",
                "fields": [{"name": "title", "value": "Shared"}],
            }
        ],
        [
            {
                "key": "source",
                "entry_type": "misc",
                "fields": [{"name": "title", "value": "Shared"}],
            }
        ],
    ]


def test_resolve_preserves_schema_for_empty_and_null_columns() -> None:
    populated = pl.DataFrame({"bibtex": ["@misc{key}"]}).select(prk.resolve("bibtex"))
    for sources in ([], [None]):
        frame = pl.DataFrame({"bibtex": pl.Series(sources, dtype=pl.String)})
        result = frame.select(prk.resolve("bibtex"))
        assert result["resolve"].to_list() == sources
        assert result.schema == populated.schema
