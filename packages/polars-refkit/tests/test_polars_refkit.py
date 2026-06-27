from __future__ import annotations

import importlib
import importlib.metadata as metadata_module
import json
import tomllib
import warnings
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
        with warnings.catch_warnings():
            warnings.filterwarnings(
                "ignore",
                message=".*overriding existing custom namespace.*",
                category=UserWarning,
            )
            reloaded = importlib.reload(prk)

    assert reloaded.__version__ == native.__version__
    with warnings.catch_warnings():
        warnings.filterwarnings(
            "ignore",
            message=".*overriding existing custom namespace.*",
            category=UserWarning,
        )
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
        bibliography=prk.full_bibliography_html("bibtex", style="apa"),
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

    collected = frame.lazy().select(citation=prk.cite("bibtex", "key")).collect()

    assert "Doe" in collected.item()


def test_polars_refkit_top_level_expressions_have_stable_default_names() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})

    result = frame.select(
        prk.entry_count("bibtex"),
        prk.keys("bibtex"),
        prk.cite("bibtex", "key"),
        prk.cite_html("bibtex", "key"),
        prk.to_hayagriva_json("bibtex"),
    )

    assert set(result.columns) == {
        "entry_count",
        "keys",
        "cite",
        "cite_html",
        "to_hayagriva_json",
    }
    assert result["entry_count"].item() == 1
    assert result["keys"].to_list()[0] == ["doe2024"]
    assert "Doe" in result["cite"].item()
    assert json.loads(result["to_hayagriva_json"].item())[0]["id"] == "doe2024"


def test_bibliography_expressions_render_all_entries_in_row() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [f"{BIBTEX}\n{SECOND_BIBTEX}"]})
    namespace = cast(Any, pl.col("bibtex")).refkit

    row = frame.select(
        html=prk.full_bibliography_html("bibtex"),
        text=prk.full_bibliography_text("bibtex"),
        rendered=prk.full_bibliography_rendered("bibtex"),
        namespace_html=namespace.full_bibliography_html(),
    ).to_dicts()[0]

    assert "Reference Work" in row["html"]
    assert "Batch References" in row["html"]
    assert "Reference Work" in row["text"]
    assert "Batch References" in row["text"]
    assert row["rendered"]["text"] == row["text"]
    assert row["rendered"]["html"] == row["html"]
    assert row["namespace_html"] == row["html"]


def test_polars_refkit_namespace_matches_function_api() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})
    namespace = cast(Any, pl.col("bibtex")).refkit

    result = frame.select(
        top_citation=prk.cite("bibtex", "key", style="apa"),
        ns_citation=namespace.cite("key", style="apa"),
        top_citation_html=prk.cite_html("bibtex", "key", style="apa"),
        ns_citation_html=namespace.cite_html("key", style="apa"),
        top_citation_rendered=prk.cite_rendered("bibtex", "key", style="apa"),
        ns_citation_rendered=namespace.cite_rendered("key", style="apa"),
        top_bibliography_html=prk.full_bibliography_html("bibtex", style="apa"),
        ns_bibliography_html=namespace.full_bibliography_html(style="apa"),
        top_bibliography_text=prk.full_bibliography_text("bibtex", style="apa"),
        ns_bibliography_text=namespace.full_bibliography_text(style="apa"),
        top_bibliography_rendered=prk.full_bibliography_rendered("bibtex", style="apa"),
        ns_bibliography_rendered=namespace.full_bibliography_rendered(style="apa"),
        top_entry_count=prk.entry_count("bibtex"),
        ns_entry_count=namespace.entry_count(),
        top_can_parse=prk.can_parse("bibtex"),
        ns_can_parse=namespace.can_parse(),
        top_has_diagnostics=prk.has_diagnostics("bibtex"),
        ns_has_diagnostics=namespace.has_diagnostics(),
        top_keys=prk.keys("bibtex"),
        ns_keys=namespace.keys(),
        top_entries=prk.entries("bibtex"),
        ns_entries=namespace.entries(),
        top_title_entries=prk.entries("bibtex", fields=("key", "title")),
        ns_title_entries=namespace.entries(fields=("key", "title")),
        top_parse_report=prk.parse_report("bibtex"),
        ns_parse_report=namespace.parse_report(),
        top_hayagriva_json=prk.to_hayagriva_json("bibtex"),
        ns_hayagriva_json=namespace.to_hayagriva_json(),
    ).to_dicts()

    row = result[0]
    assert row["top_citation"] == row["ns_citation"]
    assert row["top_citation_html"] == row["ns_citation_html"]
    assert row["top_citation_rendered"] == row["ns_citation_rendered"]
    assert row["top_bibliography_html"] == row["ns_bibliography_html"]
    assert row["top_bibliography_text"] == row["ns_bibliography_text"]
    assert row["top_bibliography_rendered"] == row["ns_bibliography_rendered"]
    assert row["top_entry_count"] == row["ns_entry_count"] == 1
    assert row["top_can_parse"] is row["ns_can_parse"] is True
    assert row["top_has_diagnostics"] is row["ns_has_diagnostics"] is False
    assert row["top_keys"] == row["ns_keys"] == ["doe2024"]
    assert row["top_entries"] == row["ns_entries"]
    assert row["top_title_entries"] == row["ns_title_entries"]
    assert row["top_parse_report"] == row["ns_parse_report"]
    assert row["top_hayagriva_json"] == row["ns_hayagriva_json"]
    assert "Doe" in row["top_citation"]
    assert "Reference Work" in row["top_bibliography_html"]
    assert row["top_entries"][0]["key"] == "doe2024"
    assert row["top_title_entries"] == [{"key": "doe2024", "title": "Reference Work"}]
    assert row["top_parse_report"]["entry_count"] == 1


def test_polars_refkit_namespace_methods_have_stable_default_names() -> None:
    import polars_refkit  # noqa: F401

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})
    namespace = cast(Any, pl.col("bibtex")).refkit

    result = frame.select(
        namespace.keys(),
        namespace.entry_count(),
        namespace.cite("key", style="apa"),
        namespace.to_hayagriva_json(),
    )

    assert set(result.columns) == {"keys", "entry_count", "cite", "to_hayagriva_json"}
    assert result["keys"].to_list()[0] == ["doe2024"]
    assert result["entry_count"].item() == 1
    assert "Doe" in result["cite"].item()
    assert json.loads(result["to_hayagriva_json"].item())[0]["id"] == "doe2024"


def test_polars_refkit_diagnostics_return_list_column() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX, "@broken{missing"]})

    result = frame.select(diagnostics=prk.diagnostics("bibtex")).to_dicts()

    assert result[0]["diagnostics"] == []
    assert "parse error" in result[1]["diagnostics"][0]


def test_polars_refkit_namespace_diagnostics_and_json() -> None:
    import polars_refkit  # noqa: F401

    frame = pl.DataFrame({"bibtex": [BIBTEX]})
    namespace = cast(Any, pl.col("bibtex")).refkit

    result = frame.select(
        diagnostics=namespace.diagnostics(),
        hayagriva_json=namespace.to_hayagriva_json(),
    ).to_dicts()[0]
    entries = cast(list[dict[str, Any]], json.loads(result["hayagriva_json"]))

    assert result["diagnostics"] == []
    assert entries[0]["id"] == "doe2024"


def test_polars_refkit_exports_normalized_json() -> None:
    import polars_refkit as prk

    row = (
        pl.DataFrame({"bibtex": [BIBTEX]})
        .select(hayagriva_json=prk.to_hayagriva_json("bibtex"))
        .to_dicts()[0]
    )

    hayagriva_entries = cast(list[dict[str, Any]], json.loads(row["hayagriva_json"]))
    assert hayagriva_entries[0]["id"] == "doe2024"
    assert hayagriva_entries[0]["key"] == "doe2024"
    assert hayagriva_entries[0]["title"] == "Reference Work"


def test_polars_refkit_accepts_literal_expressions() -> None:
    import polars_refkit as prk

    row = (
        pl.DataFrame({"key": ["doe2024"]})
        .select(
            count=prk.entry_count(pl.lit(BIBTEX)),
            citation=prk.cite(pl.lit(BIBTEX), "key"),
        )
        .to_dicts()[0]
    )

    assert row["count"] == 1
    assert "Doe" in row["citation"]


def test_polars_refkit_render_variants_return_text_html_and_structs() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})

    row = frame.select(
        citation_html=prk.cite_html("bibtex", "key", style="apa"),
        citation_struct=prk.cite_rendered("bibtex", "key", style="apa"),
        full_bibliography_text=prk.full_bibliography_text("bibtex", style="apa"),
        bibliography_struct=prk.full_bibliography_rendered("bibtex", style="apa"),
    ).to_dicts()[0]

    assert "Doe" in row["citation_html"]
    assert row["citation_struct"]["text"]
    assert row["citation_struct"]["html"]
    assert "Reference Work" in row["full_bibliography_text"]
    assert row["bibliography_struct"]["text"]
    assert row["bibliography_struct"]["html"]


def test_polars_refkit_cite_each_returns_ordered_list_outputs() -> None:
    import polars_refkit as prk

    source = f"{BIBTEX}\n{SECOND_BIBTEX}"
    frame = pl.DataFrame({"bibtex": [source], "keys": [["doe2024", "roe2022"]]})

    row = frame.select(
        citations=prk.cite_each("bibtex", "keys", style="apa"),
        citation_html=prk.cite_each_html("bibtex", "keys", style="apa"),
        rendered=prk.cite_each_rendered("bibtex", "keys", style="apa"),
    ).to_dicts()[0]

    assert len(row["citations"]) == 2
    assert "Doe" in row["citations"][0]
    assert "Roe" in row["citations"][1]
    assert "Doe" in row["citation_html"][0]
    assert row["rendered"][0]["text"] == row["citations"][0]
    assert row["rendered"][0]["html"] == row["citation_html"][0]


def test_polars_refkit_cite_group_renders_one_citation_from_key_list() -> None:
    import polars_refkit as prk

    source = f"{BIBTEX}\n{SECOND_BIBTEX}"
    frame = pl.DataFrame({"bibtex": [source], "keys": [["doe2024", "roe2022"]]})
    namespace = cast(Any, pl.col("bibtex")).refkit

    row = frame.select(
        grouped=prk.cite_group("bibtex", "keys", style="apa"),
        grouped_html=prk.cite_group_html("bibtex", "keys", style="apa"),
        grouped_rendered=prk.cite_group_rendered("bibtex", "keys", style="apa"),
        namespace_grouped=namespace.cite_group("keys", style="apa"),
        namespace_grouped_html=namespace.cite_group_html("keys", style="apa"),
        namespace_grouped_rendered=namespace.cite_group_rendered("keys", style="apa"),
    ).to_dicts()[0]

    assert isinstance(row["grouped"], str)
    assert "Doe" in row["grouped"]
    assert "Roe" in row["grouped"]
    assert row["grouped_rendered"]["text"] == row["grouped"]
    assert row["grouped_rendered"]["html"] == row["grouped_html"]
    assert row["namespace_grouped"] == row["grouped"]
    assert row["namespace_grouped_html"] == row["grouped_html"]
    assert row["namespace_grouped_rendered"] == row["grouped_rendered"]


def test_polars_refkit_cite_each_namespace_and_broadcast() -> None:
    import polars_refkit  # noqa: F401

    source = f"{BIBTEX}\n{SECOND_BIBTEX}"
    frame = pl.DataFrame({"keys": [["doe2024"], ["roe2022"]]})
    namespace = cast(Any, pl.lit(source)).refkit

    rows = frame.select(
        citations=namespace.cite_each("keys", style="apa"),
        citation_html=namespace.cite_each_html("keys", style="apa"),
        rendered=namespace.cite_each_rendered("keys", style="apa"),
    ).to_dicts()

    assert "Doe" in rows[0]["citations"][0]
    assert "Roe" in rows[1]["citations"][0]
    assert "Doe" in rows[0]["citation_html"][0]
    assert rows[1]["rendered"][0]["text"] == rows[1]["citations"][0]


def test_polars_refkit_cite_each_invalid_rows_become_nulls() -> None:
    import polars_refkit as prk

    source = f"{BIBTEX}\n{SECOND_BIBTEX}"
    frame = pl.DataFrame(
        {
            "bibtex": [source, source, "@broken{missing"],
            "keys": [["doe2024"], ["missing"], ["doe2024"]],
        }
    )

    rows = frame.select(citations=prk.cite_each("bibtex", "keys")).to_dicts()

    assert "Doe" in rows[0]["citations"][0]
    assert rows[1]["citations"] is None
    assert rows[2]["citations"] is None


def test_polars_refkit_entries_and_parse_report_are_polars_native() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX, "@broken{missing"]})

    rows = frame.select(
        entries=prk.entries("bibtex"),
        title_entries=prk.entries("bibtex", fields=("key", "title")),
        can_parse=prk.can_parse("bibtex"),
        has_diagnostics=prk.has_diagnostics("bibtex"),
        report=prk.parse_report("bibtex"),
        diagnostics=prk.diagnostics("bibtex"),
    ).to_dicts()

    entry = rows[0]["entries"][0]
    assert entry["key"] == "doe2024"
    assert entry["title"] == "Reference Work"
    assert entry["doi"] == "10.1234/refkit.polars"
    assert entry["volume"] == "7"
    assert rows[0]["title_entries"] == [{"key": "doe2024", "title": "Reference Work"}]
    assert rows[0]["can_parse"] is True
    assert rows[0]["has_diagnostics"] is False
    assert rows[0]["report"]["ok"] is True
    assert rows[0]["report"]["entry_count"] == 1
    assert rows[0]["report"]["keys"] == ["doe2024"]
    assert rows[0]["report"]["diagnostics"] == []
    assert rows[0]["diagnostics"] == []
    assert rows[1]["entries"] is None
    assert rows[1]["title_entries"] is None
    assert rows[1]["can_parse"] is False
    assert rows[1]["has_diagnostics"] is True
    assert rows[1]["report"]["ok"] is False
    assert rows[1]["report"]["entry_count"] is None
    assert rows[1]["report"]["keys"] is None
    assert "parse error" in rows[1]["report"]["diagnostics"][0]


def test_polars_refkit_recovery_modes_choose_strict_null_or_report_recovery() -> None:
    import polars_refkit as prk

    dirty = """@article{valid, author={Doe, Jane}, title={Kept}, year={2024}}
@broken{missing,
  title={No close}
"""
    frame = pl.DataFrame({"bibtex": [dirty], "key": ["valid"]})

    strict_row = frame.select(
        count=prk.entry_count("bibtex"),
        can_parse=prk.can_parse("bibtex"),
        has_diagnostics=prk.has_diagnostics("bibtex"),
        report=prk.parse_report("bibtex"),
        citation=prk.cite("bibtex", "key"),
    ).to_dicts()[0]
    report_row = frame.select(
        count=prk.entry_count("bibtex", recovery="report"),
        can_parse=prk.can_parse("bibtex", recovery="report"),
        has_diagnostics=prk.has_diagnostics("bibtex", recovery="report"),
        report=prk.parse_report("bibtex", recovery="report"),
        citation=prk.cite("bibtex", "key", recovery="report"),
    ).to_dicts()[0]

    assert strict_row["count"] is None
    assert strict_row["can_parse"] is False
    assert strict_row["has_diagnostics"] is True
    assert strict_row["report"]["ok"] is False
    assert strict_row["citation"] is None
    assert report_row["count"] == 1
    assert report_row["can_parse"] is True
    assert report_row["has_diagnostics"] is True
    assert report_row["report"]["ok"] is True
    assert report_row["report"]["keys"] == ["valid"]
    assert "ignored malformed BibTeX block" in report_row["report"]["diagnostics"][0]
    assert "Doe" in report_row["citation"]


def test_polars_refkit_rejects_cryptic_field_and_recovery_arguments() -> None:
    import polars_refkit as prk

    with pytest.raises(TypeError, match="fields must be an iterable"):
        prk.entries("bibtex", fields=cast(Any, "title"))

    with pytest.raises(ValueError, match="recovery must be"):
        prk.keys("bibtex", recovery=cast(Any, "ignore"))

    row = (
        pl.DataFrame({"bibtex": [BIBTEX]})
        .select(keys=prk.keys("bibtex", recovery="report"))
        .to_dicts()[0]
    )
    assert row["keys"] == ["doe2024"]


def test_polars_refkit_invalid_bibtex_rows_become_nulls() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX, "@broken{missing"], "key": ["doe2024", "missing"]})

    result = frame.select(
        count=prk.entry_count("bibtex"),
        keys=prk.keys("bibtex"),
        citation=prk.cite("bibtex", "key"),
        citation_struct=prk.cite_rendered("bibtex", "key"),
        citation_struct_is_null=prk.cite_rendered("bibtex", "key").is_null(),
        bibliography=prk.full_bibliography_html("bibtex"),
        bibliography_struct=prk.full_bibliography_rendered("bibtex"),
        bibliography_struct_is_null=prk.full_bibliography_rendered("bibtex").is_null(),
        hayagriva_json=prk.to_hayagriva_json("bibtex"),
    ).to_dicts()

    valid, invalid = result
    assert valid["count"] == 1
    assert valid["keys"] == ["doe2024"]
    assert "Doe" in result[0]["citation"]
    assert "Reference Work" in result[0]["bibliography"]
    assert json.loads(valid["hayagriva_json"])[0]["key"] == "doe2024"
    assert invalid["count"] is None
    assert invalid["keys"] is None
    assert invalid["citation"] is None
    assert invalid["citation_struct"] is None
    assert invalid["citation_struct_is_null"] is True
    assert invalid["bibliography"] is None
    assert invalid["bibliography_struct"] is None
    assert invalid["bibliography_struct_is_null"] is True
    assert invalid["hayagriva_json"] is None


def test_polars_refkit_missing_key_becomes_null_citation() -> None:
    import polars_refkit as prk

    result = (
        pl.DataFrame({"bibtex": [BIBTEX], "key": ["missing"]})
        .select(
            citation=prk.cite("bibtex", "key"),
            citation_struct=prk.cite_rendered("bibtex", "key"),
            citation_struct_is_null=prk.cite_rendered("bibtex", "key").is_null(),
        )
        .to_dicts()[0]
    )

    assert result["citation"] is None
    assert result["citation_struct"] is None
    assert result["citation_struct_is_null"] is True


def test_polars_refkit_unknown_style_fails_query() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})

    with pytest.raises(pl.exceptions.ComputeError, match="unknown bundled style"):
        frame.select(prk.cite("bibtex", "key", style="missing-style"))


def test_polars_refkit_rejects_non_broadcastable_inputs() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"row": [1]})

    with pytest.raises(pl.exceptions.ComputeError, match="input lengths must match"):
        frame.select(
            prk.cite(
                cast(Any, pl.Series([BIBTEX, BIBTEX])),
                cast(Any, pl.Series(["doe2024", "roe2022", "other"])),
            )
        )


def test_polars_refkit_batch_rows_are_independent() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX, SECOND_BIBTEX], "key": ["doe2024", "roe2022"]})

    result = frame.select(citation=prk.cite("bibtex", "key")).to_series().to_list()

    assert "Doe" in result[0]
    assert "Roe" in result[1]
