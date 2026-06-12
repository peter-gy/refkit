from __future__ import annotations

import importlib
import importlib.metadata as metadata_module
import json
import tomllib
from pathlib import Path
from typing import Any, cast

import polars as pl
import pytest

ROOT = Path(__file__).resolve().parents[1]
BIBTEX = """@article{doe2024,
  author = {Doe, Jane},
  title = {Reference Work},
  journal = {Journal of Citation Tests},
  year = {2024},
  volume = {7},
  doi = {10.1234/refkit.polars}
}"""
SECOND_BIBTEX = """@book{roe2022,
  author = {Roe, Richard},
  title = {Batch References},
  publisher = {Example Press},
  year = {2022}
}"""


def test_polars_refkit_imports_native_package() -> None:
    import polars_refkit as prk
    import polars_refkit._internal as native

    pyproject = tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))

    assert prk.__version__ == native.__version__ == metadata_module.version("polars-refkit")
    assert pyproject["project"]["version"] == prk.__version__


def test_polars_refkit_version_falls_back_to_native(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    import polars_refkit as prk
    import polars_refkit._internal as native

    def missing_version(name: str) -> str:
        raise metadata_module.PackageNotFoundError(name)

    with monkeypatch.context() as scoped:
        scoped.setattr(metadata_module, "version", missing_version)
        with pytest.warns(UserWarning, match="overriding existing custom namespace"):
            reloaded = importlib.reload(prk)

    assert reloaded.__version__ == native.__version__
    with pytest.warns(UserWarning, match="overriding existing custom namespace"):
        importlib.reload(prk)


def test_polars_refkit_rejects_unknown_attribute() -> None:
    import polars_refkit as prk

    missing_attribute = "does_not_exist"
    with pytest.raises(AttributeError, match="does_not_exist"):
        getattr(prk, missing_attribute)


def test_polars_refkit_expressions_parse_and_render_bibtex_rows() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})

    result = frame.select(
        citation=prk.cite("bibtex", "key", style="apa"),
        bibliography=prk.bibliography_html("bibtex", style="apa"),
        count=prk.entry_count("bibtex"),
        keys=prk.keys("bibtex"),
    ).to_dicts()

    row = result[0]
    assert "Doe" in row["citation"]
    assert "Reference Work" in row["bibliography"]
    assert row["count"] == 1
    assert row["keys"] == ["doe2024"]


def test_polars_refkit_lazy_expressions_collect() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})

    collected = cast(
        pl.DataFrame,
        frame.lazy().select(citation=prk.cite_bibtex("bibtex", "key")).collect(),
    )

    assert "Doe" in collected.item()


def test_polars_refkit_top_level_expressions_have_stable_default_names() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})

    result = frame.select(
        prk.bibtex_entry_count("bibtex"),
        prk.bibtex_keys("bibtex"),
        prk.cite("bibtex", "key"),
        prk.cite_html("bibtex", "key"),
        prk.entries_json("bibtex"),
    )

    assert result.columns == [
        "bibtex_entry_count",
        "bibtex_keys",
        "cite",
        "cite_html",
        "entries_json",
    ]
    assert result["bibtex_entry_count"].item() == 1
    assert result["bibtex_keys"].to_list()[0] == ["doe2024"]
    assert "Doe" in result["cite"].item()


def test_polars_refkit_bibliography_all_keyword_remains_supported() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX]})
    namespace = cast(Any, pl.col("bibtex")).refkit

    row = frame.select(
        explicit=prk.bibliography_bibtex("bibtex", all=False),
        explicit_text=prk.bibliography_bibtex_text("bibtex", all=False),
        explicit_rendered=prk.bibliography_bibtex_rendered("bibtex", all=False),
        namespace=namespace.bibliography(all=False),
    ).to_dicts()[0]

    assert row == {
        "explicit": "",
        "explicit_text": "",
        "explicit_rendered": {"text": "", "html": ""},
        "namespace": "",
    }


def test_polars_refkit_namespace_matches_function_api() -> None:
    import polars_refkit  # noqa: F401

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})
    namespace = cast(Any, pl.col("bibtex")).refkit

    result = frame.select(
        citation=namespace.cite("key", style="apa"),
        citation_html=namespace.cite_html("key", style="apa"),
        citation_rendered=namespace.cite_rendered("key", style="apa"),
        bibliography=namespace.bibliography(style="apa"),
        bibliography_html=namespace.bibliography_html(style="apa"),
        bibliography_text=namespace.bibliography_text(style="apa"),
        bibliography_rendered=namespace.bibliography_rendered(style="apa"),
        entry_count=namespace.entry_count(),
        is_valid=namespace.is_valid(),
        keys=namespace.keys(),
        entries=namespace.entries(),
        title_entries=namespace.entries(fields=("key", "title")),
        parse_report=namespace.parse_report(),
    ).to_dicts()

    row = result[0]
    assert "Doe" in row["citation"]
    assert "Doe" in row["citation_html"]
    assert row["citation_rendered"]["text"] == row["citation"]
    assert "Reference Work" in row["bibliography"]
    assert "Reference Work" in row["bibliography_html"]
    assert "Reference Work" in row["bibliography_text"]
    assert row["bibliography_rendered"]["html"] == row["bibliography_html"]
    assert row["entry_count"] == 1
    assert row["is_valid"] is True
    assert row["keys"] == ["doe2024"]
    assert row["entries"][0]["key"] == "doe2024"
    assert row["title_entries"] == [{"key": "doe2024", "title": "Reference Work"}]
    assert row["parse_report"]["entry_count"] == 1


def test_polars_refkit_namespace_methods_have_stable_default_names() -> None:
    import polars_refkit  # noqa: F401

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})
    namespace = cast(Any, pl.col("bibtex")).refkit

    result = frame.select(
        namespace.keys(),
        namespace.entry_count(),
        namespace.cite("key", style="apa"),
        namespace.to_csl_json(),
    )

    assert result.columns == ["keys", "entry_count", "cite", "to_csl_json"]
    assert result["keys"].to_list()[0] == ["doe2024"]
    assert result["entry_count"].item() == 1
    assert "Doe" in result["cite"].item()
    assert json.loads(result["to_csl_json"].item())[0]["id"] == "doe2024"


def test_polars_refkit_diagnostics_return_list_column() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX, "@broken{missing"]})

    result = frame.select(diagnostics=prk.bibtex_diagnostics("bibtex")).to_dicts()

    assert result[0]["diagnostics"] == []
    assert "biblatex parse error" in result[1]["diagnostics"][0]


def test_polars_refkit_namespace_diagnostics_and_json() -> None:
    import polars_refkit  # noqa: F401

    frame = pl.DataFrame({"bibtex": [BIBTEX]})
    namespace = cast(Any, pl.col("bibtex")).refkit

    result = frame.select(
        diagnostics=namespace.diagnostics(),
        csl_json=namespace.to_csl_json(),
        hayagriva_json=namespace.to_hayagriva_json(),
        entries_json=namespace.entries_json(),
    ).to_dicts()[0]
    entries = cast(list[dict[str, Any]], json.loads(result["hayagriva_json"]))

    assert result["diagnostics"] == []
    assert entries[0]["id"] == "doe2024"
    assert json.loads(result["csl_json"])[0]["id"] == "doe2024"
    assert json.loads(result["entries_json"])[0]["id"] == "doe2024"


def test_polars_refkit_exports_normalized_json() -> None:
    import polars_refkit as prk

    row = (
        pl.DataFrame({"bibtex": [BIBTEX]})
        .select(
            entries_json=prk.entries_json("bibtex"),
            hayagriva_json=prk.bibtex_to_hayagriva_json("bibtex"),
        )
        .to_dicts()[0]
    )

    assert row["hayagriva_json"] == row["entries_json"]
    entries = cast(list[dict[str, Any]], json.loads(row["entries_json"]))
    assert entries[0]["id"] == "doe2024"
    assert entries[0]["key"] == "doe2024"
    assert entries[0]["title"] == "Reference Work"


def test_polars_refkit_accepts_literal_expressions() -> None:
    import polars_refkit as prk

    result = pl.DataFrame({"row": [1]}).select(prk.bibtex_entry_count(pl.Series([BIBTEX]))).item()

    assert result == 1


def test_polars_refkit_render_variants_return_text_html_and_structs() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})

    row = frame.select(
        citation_html=prk.cite_html("bibtex", "key", style="apa"),
        citation_struct=prk.cite_rendered("bibtex", "key", style="apa"),
        bib_citation_html=prk.cite_bibtex_html("bibtex", "key", style="apa"),
        bib_citation_struct=prk.cite_bibtex_rendered("bibtex", "key", style="apa"),
        bibliography_text=prk.bibliography_text("bibtex", style="apa"),
        bibliography_struct=prk.bibliography_rendered("bibtex", style="apa"),
        bib_bibliography_text=prk.bibliography_bibtex_text("bibtex", style="apa"),
        bib_bibliography_struct=prk.bibliography_bibtex_rendered("bibtex", style="apa"),
    ).to_dicts()[0]

    assert "Doe" in row["citation_html"]
    assert row["citation_struct"]["text"]
    assert row["citation_struct"]["html"]
    assert row["bib_citation_struct"] == row["citation_struct"]
    assert "Reference Work" in row["bibliography_text"]
    assert row["bibliography_struct"]["text"]
    assert row["bibliography_struct"]["html"]
    assert row["bib_bibliography_text"] == row["bibliography_text"]
    assert row["bib_bibliography_struct"] == row["bibliography_struct"]


def test_polars_refkit_cite_sequence_returns_ordered_list_outputs() -> None:
    import polars_refkit as prk

    source = f"{BIBTEX}\n{SECOND_BIBTEX}"
    frame = pl.DataFrame({"bibtex": [source], "keys": [["doe2024", "roe2022"]]})

    row = frame.select(
        citations=prk.cite_sequence("bibtex", "keys", style="apa"),
        citation_html=prk.cite_sequence_html("bibtex", "keys", style="apa"),
        rendered=prk.cite_sequence_rendered("bibtex", "keys", style="apa"),
        bib_citations=prk.cite_bibtex_sequence("bibtex", "keys", style="apa"),
        bib_citation_html=prk.cite_bibtex_sequence_html("bibtex", "keys", style="apa"),
        bib_rendered=prk.cite_bibtex_sequence_rendered("bibtex", "keys", style="apa"),
    ).to_dicts()[0]

    assert len(row["citations"]) == 2
    assert "Doe" in row["citations"][0]
    assert "Roe" in row["citations"][1]
    assert "Doe" in row["citation_html"][0]
    assert row["rendered"][0]["text"] == row["citations"][0]
    assert row["rendered"][0]["html"] == row["citation_html"][0]
    assert row["bib_citations"] == row["citations"]
    assert row["bib_citation_html"] == row["citation_html"]
    assert row["bib_rendered"] == row["rendered"]


def test_polars_refkit_cite_sequence_namespace_and_broadcast() -> None:
    import polars_refkit  # noqa: F401

    source = f"{BIBTEX}\n{SECOND_BIBTEX}"
    frame = pl.DataFrame({"keys": [["doe2024"], ["roe2022"]]})
    namespace = cast(Any, pl.lit(source)).refkit

    rows = frame.select(
        citations=namespace.cite_sequence("keys", style="apa"),
        citation_html=namespace.cite_sequence_html("keys", style="apa"),
        rendered=namespace.cite_sequence_rendered("keys", style="apa"),
    ).to_dicts()

    assert "Doe" in rows[0]["citations"][0]
    assert "Roe" in rows[1]["citations"][0]
    assert "Doe" in rows[0]["citation_html"][0]
    assert rows[1]["rendered"][0]["text"] == rows[1]["citations"][0]


def test_polars_refkit_cite_sequence_invalid_rows_become_nulls() -> None:
    import polars_refkit as prk

    source = f"{BIBTEX}\n{SECOND_BIBTEX}"
    frame = pl.DataFrame(
        {
            "bibtex": [source, source, "@broken{missing"],
            "keys": [["doe2024"], ["missing"], ["doe2024"]],
        }
    )

    rows = frame.select(citations=prk.cite_sequence("bibtex", "keys")).to_dicts()

    assert "Doe" in rows[0]["citations"][0]
    assert rows[1]["citations"] is None
    assert rows[2]["citations"] is None


def test_polars_refkit_entries_and_parse_report_are_polars_native() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX, "@broken{missing"]})

    rows = frame.select(
        entries=prk.entries("bibtex"),
        title_entries=prk.bibtex_entries("bibtex", fields=("key", "title")),
        valid=prk.is_valid("bibtex"),
        bib_valid=prk.bibtex_is_valid("bibtex"),
        report=prk.parse_report("bibtex"),
        bib_report=prk.bibtex_parse_report("bibtex"),
        strict_diagnostics=prk.diagnostics("bibtex", strict=True),
    ).to_dicts()

    assert rows[0]["entries"][0] == {
        "key": "doe2024",
        "entry_type": "Article",
        "title": "Reference Work",
        "doi": "10.1234/refkit.polars",
        "volume": "7",
    }
    assert rows[0]["title_entries"] == [{"key": "doe2024", "title": "Reference Work"}]
    assert rows[0]["valid"] is True
    assert rows[0]["bib_valid"] is True
    assert rows[0]["report"]["ok"] is True
    assert rows[0]["report"]["entry_count"] == 1
    assert rows[0]["report"]["keys"] == ["doe2024"]
    assert rows[0]["report"]["diagnostics"] == []
    assert rows[0]["bib_report"] == rows[0]["report"]
    assert rows[0]["strict_diagnostics"] == []
    assert rows[1]["entries"] is None
    assert rows[1]["title_entries"] is None
    assert rows[1]["valid"] is False
    assert rows[1]["report"]["ok"] is False
    assert rows[1]["report"]["entry_count"] is None
    assert rows[1]["report"]["keys"] is None
    assert "biblatex parse error" in rows[1]["report"]["diagnostics"][0]


def test_polars_refkit_invalid_bibtex_rows_become_nulls() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX, "@broken{missing"], "key": ["doe2024", "missing"]})

    result = frame.select(
        count=prk.bibtex_entry_count("bibtex"),
        keys=prk.bibtex_keys("bibtex"),
        citation=prk.cite_bibtex("bibtex", "key"),
        citation_struct=prk.cite_rendered("bibtex", "key"),
        citation_struct_is_null=prk.cite_rendered("bibtex", "key").is_null(),
        bibliography=prk.bibliography_bibtex("bibtex"),
        bibliography_struct=prk.bibliography_rendered("bibtex"),
        bibliography_struct_is_null=prk.bibliography_rendered("bibtex").is_null(),
        csl_json=prk.bibtex_to_csl_json("bibtex"),
    ).to_dicts()

    valid, invalid = result
    assert valid["count"] == 1
    assert valid["keys"] == ["doe2024"]
    assert "Doe" in result[0]["citation"]
    assert "Reference Work" in result[0]["bibliography"]
    assert json.loads(valid["csl_json"])[0]["key"] == "doe2024"
    assert invalid == {
        "count": None,
        "keys": None,
        "citation": None,
        "citation_struct": None,
        "citation_struct_is_null": True,
        "bibliography": None,
        "bibliography_struct": None,
        "bibliography_struct_is_null": True,
        "csl_json": None,
    }


def test_polars_refkit_missing_key_becomes_null_citation() -> None:
    import polars_refkit as prk

    result = (
        pl.DataFrame({"bibtex": [BIBTEX], "key": ["missing"]})
        .select(
            citation=prk.cite_bibtex("bibtex", "key"),
            citation_struct=prk.cite_rendered("bibtex", "key"),
            citation_struct_is_null=prk.cite_rendered("bibtex", "key").is_null(),
        )
        .to_dicts()[0]
    )

    assert result == {
        "citation": None,
        "citation_struct": None,
        "citation_struct_is_null": True,
    }


def test_polars_refkit_unknown_style_fails_query() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})

    with pytest.raises(pl.exceptions.ComputeError, match="unknown bundled style"):
        frame.select(prk.cite_bibtex("bibtex", "key", style="missing-style"))


def test_polars_refkit_rejects_non_broadcastable_inputs() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"row": [1]})

    with pytest.raises(pl.exceptions.ComputeError, match="input lengths must match"):
        frame.select(
            prk.cite_bibtex(
                pl.Series([BIBTEX, BIBTEX]),
                pl.Series(["doe2024", "roe2022", "other"]),
            )
        )


def test_polars_refkit_batch_rows_are_independent() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX, SECOND_BIBTEX], "key": ["doe2024", "roe2022"]})

    result = frame.select(citation=prk.cite_bibtex("bibtex", "key")).to_series().to_list()

    assert "Doe" in result[0]
    assert "Roe" in result[1]
