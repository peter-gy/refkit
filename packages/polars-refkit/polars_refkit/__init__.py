"""Polars expressions for BibTeX citation workflows."""

from __future__ import annotations

from importlib.metadata import PackageNotFoundError
from importlib.metadata import version as _metadata_version
from pathlib import Path
from typing import Any

import polars as pl
from polars.plugins import register_plugin_function

from ._internal import __version__ as _native_version

PLUGIN_PATH = Path(__file__).parent

try:
    __version__ = _metadata_version("polars-refkit")
except PackageNotFoundError:
    __version__ = _native_version

__all__ = [
    "__version__",
    "bibliography_bibtex",
    "bibtex_diagnostics",
    "bibtex_entry_count",
    "bibtex_keys",
    "bibtex_to_csl_json",
    "cite_bibtex",
]


def cite_bibtex(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render one citation from each BibTeX row and key row."""

    return _register(
        "cite_bibtex",
        [bibtex, key],
        kwargs={"style": style, "locale": locale, "strict": strict, "all": True},
        output_name="cite_bibtex",
    )


def bibliography_bibtex(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    all: bool = True,
    strict: bool = False,
) -> pl.Expr:
    """Render an HTML bibliography from each BibTeX row."""

    return _register(
        "bibliography_bibtex",
        [bibtex],
        kwargs={"style": style, "locale": locale, "strict": strict, "all": all},
        output_name="bibliography_bibtex",
    )


def bibtex_entry_count(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return the number of normalized entries in each BibTeX row."""

    return _register_parse("bibtex_entry_count", bibtex, strict=strict)


def bibtex_keys(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return citation keys as a list column for each BibTeX row."""

    return _register_parse("bibtex_keys", bibtex, strict=strict)


def bibtex_diagnostics(bibtex: Any) -> pl.Expr:
    """Return parser diagnostics as a list column for each BibTeX row."""

    return _register(
        "bibtex_diagnostics",
        [bibtex],
        kwargs=None,
        output_name="bibtex_diagnostics",
    )


def bibtex_to_csl_json(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return normalized entry JSON for each BibTeX row."""

    return _register_parse("bibtex_to_csl_json", bibtex, strict=strict)


@pl.api.register_expr_namespace("refkit")
class RefkitExprNamespace:
    """BibTeX expressions available from `pl.Expr.refkit`."""

    def __init__(self, expr: pl.Expr) -> None:
        self._expr = expr

    def cite(
        self,
        key: Any,
        *,
        style: str = "apa",
        locale: str = "en-US",
        strict: bool = False,
    ) -> pl.Expr:
        """Render one citation from each BibTeX row and key row."""

        return cite_bibtex(self._expr, key, style=style, locale=locale, strict=strict).alias("cite")

    def bibliography(
        self,
        *,
        style: str = "apa",
        locale: str = "en-US",
        all: bool = True,
        strict: bool = False,
    ) -> pl.Expr:
        """Render an HTML bibliography from each BibTeX row."""

        return bibliography_bibtex(
            self._expr, style=style, locale=locale, all=all, strict=strict
        ).alias("bibliography")

    def entry_count(self, *, strict: bool = False) -> pl.Expr:
        """Return the number of normalized entries in each BibTeX row."""

        return bibtex_entry_count(self._expr, strict=strict).alias("entry_count")

    def keys(self, *, strict: bool = False) -> pl.Expr:
        """Return citation keys as a list column for each BibTeX row."""

        return bibtex_keys(self._expr, strict=strict).alias("keys")

    def diagnostics(self) -> pl.Expr:
        """Return parser diagnostics as a list column for each BibTeX row."""

        return bibtex_diagnostics(self._expr).alias("diagnostics")

    def to_csl_json(self, *, strict: bool = False) -> pl.Expr:
        """Return normalized entry JSON for each BibTeX row."""

        return bibtex_to_csl_json(self._expr, strict=strict).alias("to_csl_json")


def _register_parse(function_name: str, bibtex: Any, *, strict: bool) -> pl.Expr:
    return _register(
        function_name,
        [bibtex],
        kwargs={"strict": strict},
        output_name=function_name,
    )


def _register(
    function_name: str,
    args: list[Any],
    *,
    kwargs: dict[str, Any] | None,
    output_name: str,
) -> pl.Expr:
    return register_plugin_function(
        plugin_path=PLUGIN_PATH,
        function_name=function_name,
        args=[_parse_into_expr(arg) for arg in args],
        kwargs=kwargs,
        is_elementwise=True,
    ).alias(output_name)


def _parse_into_expr(expr: Any) -> pl.Expr:
    if isinstance(expr, pl.Expr):
        return expr
    if isinstance(expr, str):
        return pl.col(expr)
    return pl.lit(expr)


def __getattr__(name: str) -> Any:
    raise AttributeError(f"module 'polars_refkit' has no attribute {name!r}")
