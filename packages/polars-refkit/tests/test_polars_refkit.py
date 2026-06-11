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
        citation=prk.cite_bibtex("bibtex", "key", style="apa"),
        bibliography=prk.bibliography_bibtex("bibtex", style="apa"),
        count=prk.bibtex_entry_count("bibtex"),
        keys=prk.bibtex_keys("bibtex"),
    ).to_dicts()

    row = result[0]
    assert "Doe" in row["citation"]
    assert "Reference Work" in row["bibliography"]
    assert row["count"] == 1
    assert row["keys"] == ["doe2024"]


def test_polars_refkit_lazy_expressions_collect() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})

    collected = cast(pl.DataFrame, frame.lazy().select(prk.cite_bibtex("bibtex", "key")).collect())

    assert "Doe" in collected.item()


def test_polars_refkit_namespace_matches_function_api() -> None:
    import polars_refkit  # noqa: F401

    frame = pl.DataFrame({"bibtex": [BIBTEX], "key": ["doe2024"]})
    namespace = cast(Any, pl.col("bibtex")).refkit

    result = frame.select(
        namespace.cite("key", style="apa"),
        namespace.bibliography(style="apa"),
        namespace.entry_count(),
        namespace.keys(),
    ).to_dicts()

    row = result[0]
    assert "Doe" in row["cite"]
    assert "Reference Work" in row["bibliography"]
    assert row["entry_count"] == 1
    assert row["keys"] == ["doe2024"]


def test_polars_refkit_diagnostics_return_list_column() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX, "@broken{missing"]})

    result = frame.select(prk.bibtex_diagnostics("bibtex")).to_dicts()

    assert result[0]["bibtex_diagnostics"] == []
    assert "biblatex parse error" in result[1]["bibtex_diagnostics"][0]


def test_polars_refkit_namespace_diagnostics_and_json() -> None:
    import polars_refkit  # noqa: F401

    frame = pl.DataFrame({"bibtex": [BIBTEX]})
    namespace = cast(Any, pl.col("bibtex")).refkit

    result = frame.select(namespace.diagnostics(), namespace.to_csl_json()).to_dicts()[0]
    entries = cast(list[dict[str, Any]], json.loads(result["to_csl_json"]))

    assert result["diagnostics"] == []
    assert entries[0]["id"] == "doe2024"


def test_polars_refkit_exports_normalized_json() -> None:
    import polars_refkit as prk

    payload = pl.DataFrame({"bibtex": [BIBTEX]}).select(prk.bibtex_to_csl_json("bibtex")).item()

    entries = cast(list[dict[str, Any]], json.loads(payload))
    assert entries[0]["id"] == "doe2024"
    assert entries[0]["key"] == "doe2024"
    assert entries[0]["title"] == "Reference Work"


def test_polars_refkit_accepts_literal_expressions() -> None:
    import polars_refkit as prk

    result = pl.DataFrame({"row": [1]}).select(prk.bibtex_entry_count(pl.Series([BIBTEX]))).item()

    assert result == 1


def test_polars_refkit_invalid_bibtex_rows_become_nulls() -> None:
    import polars_refkit as prk

    frame = pl.DataFrame({"bibtex": [BIBTEX, "@broken{missing"], "key": ["doe2024", "missing"]})

    result = frame.select(
        prk.bibtex_entry_count("bibtex"),
        prk.bibtex_keys("bibtex"),
        prk.cite_bibtex("bibtex", "key"),
        prk.bibliography_bibtex("bibtex"),
        prk.bibtex_to_csl_json("bibtex"),
    ).to_dicts()

    valid, invalid = result
    assert valid["bibtex_entry_count"] == 1
    assert valid["bibtex_keys"] == ["doe2024"]
    assert "Doe" in result[0]["cite_bibtex"]
    assert "Reference Work" in result[0]["bibliography_bibtex"]
    assert json.loads(valid["bibtex_to_csl_json"])[0]["key"] == "doe2024"
    assert invalid == {
        "bibtex_entry_count": None,
        "bibtex_keys": None,
        "cite_bibtex": None,
        "bibliography_bibtex": None,
        "bibtex_to_csl_json": None,
    }


def test_polars_refkit_missing_key_becomes_null_citation() -> None:
    import polars_refkit as prk

    result = (
        pl.DataFrame({"bibtex": [BIBTEX], "key": ["missing"]})
        .select(prk.cite_bibtex("bibtex", "key"))
        .item()
    )

    assert result is None


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

    result = frame.select(prk.cite_bibtex("bibtex", "key")).to_series().to_list()

    assert "Doe" in result[0]
    assert "Roe" in result[1]
