"""Polars expressions for BibTeX citation workflows."""

from __future__ import annotations

from collections.abc import Iterable
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
    "bibliography_html",
    "bibliography_bibtex",
    "bibliography_bibtex_rendered",
    "bibliography_bibtex_text",
    "bibliography_rendered",
    "bibliography_text",
    "bibtex_diagnostics",
    "bibtex_entries",
    "bibtex_entry_count",
    "bibtex_is_valid",
    "bibtex_keys",
    "bibtex_parse_report",
    "bibtex_to_hayagriva_json",
    "cite",
    "cite_bibtex",
    "cite_bibtex_html",
    "cite_bibtex_rendered",
    "cite_bibtex_sequence",
    "cite_bibtex_sequence_html",
    "cite_bibtex_sequence_rendered",
    "cite_html",
    "cite_rendered",
    "cite_sequence",
    "cite_sequence_html",
    "cite_sequence_rendered",
    "diagnostics",
    "entries",
    "entries_json",
    "entry_count",
    "is_valid",
    "keys",
    "parse_report",
]


def cite(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render one citation as plain text from each BibTeX row and key row."""

    return _cite_expr(
        "cite_bibtex",
        bibtex,
        key,
        style=style,
        locale=locale,
        strict=strict,
        output_name="cite",
    )


def cite_bibtex(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render one citation from each BibTeX row and key row."""

    return _cite_expr(
        "cite_bibtex",
        bibtex,
        key,
        style=style,
        locale=locale,
        strict=strict,
        output_name="cite_bibtex",
    )


def cite_html(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render one citation as HTML from each BibTeX row and key row."""

    return _cite_expr(
        "cite_bibtex_html",
        bibtex,
        key,
        style=style,
        locale=locale,
        strict=strict,
        output_name="cite_html",
    )


def cite_bibtex_html(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render one citation as HTML from each BibTeX row and key row."""

    return _cite_expr(
        "cite_bibtex_html",
        bibtex,
        key,
        style=style,
        locale=locale,
        strict=strict,
        output_name="cite_bibtex_html",
    )


def cite_rendered(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render one citation as a `{text, html}` struct."""

    return _cite_expr(
        "cite_bibtex_rendered",
        bibtex,
        key,
        style=style,
        locale=locale,
        strict=strict,
        output_name="cite_rendered",
    )


def cite_bibtex_rendered(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render one citation as a `{text, html}` struct."""

    return _cite_expr(
        "cite_bibtex_rendered",
        bibtex,
        key,
        style=style,
        locale=locale,
        strict=strict,
        output_name="cite_bibtex_rendered",
    )


def cite_sequence(
    bibtex: Any,
    keys: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render ordered citation texts from one BibTeX row and one list-of-keys row."""

    return _cite_sequence_expr(
        "cite_bibtex_sequence",
        bibtex,
        keys,
        style=style,
        locale=locale,
        strict=strict,
        output_name="cite_sequence",
    )


def cite_bibtex_sequence(
    bibtex: Any,
    keys: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render ordered citation texts from one BibTeX row and one list-of-keys row."""

    return _cite_sequence_expr(
        "cite_bibtex_sequence",
        bibtex,
        keys,
        style=style,
        locale=locale,
        strict=strict,
        output_name="cite_bibtex_sequence",
    )


def cite_sequence_html(
    bibtex: Any,
    keys: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render ordered citation HTML from one BibTeX row and one list-of-keys row."""

    return _cite_sequence_expr(
        "cite_bibtex_sequence_html",
        bibtex,
        keys,
        style=style,
        locale=locale,
        strict=strict,
        output_name="cite_sequence_html",
    )


def cite_bibtex_sequence_html(
    bibtex: Any,
    keys: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render ordered citation HTML from one BibTeX row and one list-of-keys row."""

    return _cite_sequence_expr(
        "cite_bibtex_sequence_html",
        bibtex,
        keys,
        style=style,
        locale=locale,
        strict=strict,
        output_name="cite_bibtex_sequence_html",
    )


def cite_sequence_rendered(
    bibtex: Any,
    keys: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render ordered citations as a list of `{text, html}` structs."""

    return _cite_sequence_expr(
        "cite_bibtex_sequence_rendered",
        bibtex,
        keys,
        style=style,
        locale=locale,
        strict=strict,
        output_name="cite_sequence_rendered",
    )


def cite_bibtex_sequence_rendered(
    bibtex: Any,
    keys: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render ordered citations as a list of `{text, html}` structs."""

    return _cite_sequence_expr(
        "cite_bibtex_sequence_rendered",
        bibtex,
        keys,
        style=style,
        locale=locale,
        strict=strict,
        output_name="cite_bibtex_sequence_rendered",
    )


def bibliography_html(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render an HTML bibliography from each BibTeX row."""

    return _bibliography_expr(
        "bibliography_bibtex",
        bibtex,
        style=style,
        locale=locale,
        strict=strict,
        output_name="bibliography_html",
    )


def bibliography_bibtex(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render an HTML bibliography from each BibTeX row."""

    return _bibliography_expr(
        "bibliography_bibtex",
        bibtex,
        style=style,
        locale=locale,
        strict=strict,
        output_name="bibliography_bibtex",
    )


def bibliography_text(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render a plain-text bibliography from each BibTeX row."""

    return _bibliography_expr(
        "bibliography_bibtex_text",
        bibtex,
        style=style,
        locale=locale,
        strict=strict,
        output_name="bibliography_text",
    )


def bibliography_bibtex_text(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render a plain-text bibliography from each BibTeX row."""

    return _bibliography_expr(
        "bibliography_bibtex_text",
        bibtex,
        style=style,
        locale=locale,
        strict=strict,
        output_name="bibliography_bibtex_text",
    )


def bibliography_rendered(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render a bibliography as a `{text, html}` struct."""

    return _bibliography_expr(
        "bibliography_bibtex_rendered",
        bibtex,
        style=style,
        locale=locale,
        strict=strict,
        output_name="bibliography_rendered",
    )


def bibliography_bibtex_rendered(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr:
    """Render a bibliography as a `{text, html}` struct."""

    return _bibliography_expr(
        "bibliography_bibtex_rendered",
        bibtex,
        style=style,
        locale=locale,
        strict=strict,
        output_name="bibliography_bibtex_rendered",
    )


def entry_count(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return the number of normalized entries in each BibTeX row."""

    return _parse_expr(
        "bibtex_entry_count",
        bibtex,
        strict=strict,
        output_name="entry_count",
    )


def bibtex_entry_count(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return the number of normalized entries in each BibTeX row."""

    return _parse_expr(
        "bibtex_entry_count",
        bibtex,
        strict=strict,
        output_name="bibtex_entry_count",
    )


def bibtex_is_valid(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return whether each BibTeX row parses into a normalized library."""

    return _parse_expr(
        "bibtex_is_valid",
        bibtex,
        strict=strict,
        output_name="bibtex_is_valid",
    )


def is_valid(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return whether each BibTeX row parses into a normalized library."""

    return _parse_expr("bibtex_is_valid", bibtex, strict=strict, output_name="is_valid")


def keys(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return citation keys as a list column for each BibTeX row."""

    return _parse_expr("bibtex_keys", bibtex, strict=strict, output_name="keys")


def bibtex_keys(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return citation keys as a list column for each BibTeX row."""

    return _parse_expr(
        "bibtex_keys",
        bibtex,
        strict=strict,
        output_name="bibtex_keys",
    )


def entries(
    bibtex: Any,
    *,
    fields: Iterable[str] = ("key", "entry_type", "title", "doi", "volume"),
    strict: bool = False,
) -> pl.Expr:
    """Return normalized entry records as `List[Struct]` for each BibTeX row."""

    return _entries_expr(bibtex, fields=fields, strict=strict, output_name="entries")


def bibtex_entries(
    bibtex: Any,
    *,
    fields: Iterable[str] = ("key", "entry_type", "title", "doi", "volume"),
    strict: bool = False,
) -> pl.Expr:
    """Return normalized entry records as `List[Struct]` for each BibTeX row."""

    return _entries_expr(bibtex, fields=fields, strict=strict, output_name="bibtex_entries")


def bibtex_diagnostics(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return parser diagnostics as a list column for each BibTeX row."""

    return _diagnostics_expr(bibtex, strict=strict, output_name="bibtex_diagnostics")


def diagnostics(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return parser diagnostics as a list column for each BibTeX row."""

    return _diagnostics_expr(bibtex, strict=strict, output_name="diagnostics")


def parse_report(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return `{ok, entry_count, keys, diagnostics}` from one parse per row."""

    return _parse_expr(
        "bibtex_parse_report",
        bibtex,
        strict=strict,
        output_name="parse_report",
    )


def bibtex_parse_report(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return `{ok, entry_count, keys, diagnostics}` from one parse per row."""

    return _parse_expr(
        "bibtex_parse_report",
        bibtex,
        strict=strict,
        output_name="bibtex_parse_report",
    )


def entries_json(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return normalized Hayagriva entry JSON for each BibTeX row."""

    return _parse_expr(
        "bibtex_to_hayagriva_json",
        bibtex,
        strict=strict,
        output_name="entries_json",
    )


def bibtex_to_hayagriva_json(bibtex: Any, *, strict: bool = False) -> pl.Expr:
    """Return normalized Hayagriva entry JSON for each BibTeX row."""

    return _parse_expr(
        "bibtex_to_hayagriva_json",
        bibtex,
        strict=strict,
        output_name="bibtex_to_hayagriva_json",
    )


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

        return _cite_expr(
            "cite_bibtex",
            self._expr,
            key,
            style=style,
            locale=locale,
            strict=strict,
            output_name="cite",
        )

    def cite_html(
        self,
        key: Any,
        *,
        style: str = "apa",
        locale: str = "en-US",
        strict: bool = False,
    ) -> pl.Expr:
        """Render one citation as HTML from each BibTeX row and key row."""

        return _cite_expr(
            "cite_bibtex_html",
            self._expr,
            key,
            style=style,
            locale=locale,
            strict=strict,
            output_name="cite_html",
        )

    def cite_rendered(
        self,
        key: Any,
        *,
        style: str = "apa",
        locale: str = "en-US",
        strict: bool = False,
    ) -> pl.Expr:
        """Render one citation as a `{text, html}` struct."""

        return _cite_expr(
            "cite_bibtex_rendered",
            self._expr,
            key,
            style=style,
            locale=locale,
            strict=strict,
            output_name="cite_rendered",
        )

    def cite_sequence(
        self,
        keys: Any,
        *,
        style: str = "apa",
        locale: str = "en-US",
        strict: bool = False,
    ) -> pl.Expr:
        """Render ordered citation texts from one BibTeX row and one list-of-keys row."""

        return _cite_sequence_expr(
            "cite_bibtex_sequence",
            self._expr,
            keys,
            style=style,
            locale=locale,
            strict=strict,
            output_name="cite_sequence",
        )

    def cite_sequence_html(
        self,
        keys: Any,
        *,
        style: str = "apa",
        locale: str = "en-US",
        strict: bool = False,
    ) -> pl.Expr:
        """Render ordered citation HTML from one BibTeX row and one list-of-keys row."""

        return _cite_sequence_expr(
            "cite_bibtex_sequence_html",
            self._expr,
            keys,
            style=style,
            locale=locale,
            strict=strict,
            output_name="cite_sequence_html",
        )

    def cite_sequence_rendered(
        self,
        keys: Any,
        *,
        style: str = "apa",
        locale: str = "en-US",
        strict: bool = False,
    ) -> pl.Expr:
        """Render ordered citations as a list of `{text, html}` structs."""

        return _cite_sequence_expr(
            "cite_bibtex_sequence_rendered",
            self._expr,
            keys,
            style=style,
            locale=locale,
            strict=strict,
            output_name="cite_sequence_rendered",
        )

    def bibliography(
        self,
        *,
        style: str = "apa",
        locale: str = "en-US",
        strict: bool = False,
    ) -> pl.Expr:
        """Render an HTML bibliography from each BibTeX row."""

        return _bibliography_expr(
            "bibliography_bibtex",
            self._expr,
            style=style,
            locale=locale,
            strict=strict,
            output_name="bibliography",
        )

    def bibliography_html(
        self,
        *,
        style: str = "apa",
        locale: str = "en-US",
        strict: bool = False,
    ) -> pl.Expr:
        """Render an HTML bibliography from each BibTeX row."""

        return _bibliography_expr(
            "bibliography_bibtex",
            self._expr,
            style=style,
            locale=locale,
            strict=strict,
            output_name="bibliography_html",
        )

    def bibliography_text(
        self,
        *,
        style: str = "apa",
        locale: str = "en-US",
        strict: bool = False,
    ) -> pl.Expr:
        """Render a plain-text bibliography from each BibTeX row."""

        return _bibliography_expr(
            "bibliography_bibtex_text",
            self._expr,
            style=style,
            locale=locale,
            strict=strict,
            output_name="bibliography_text",
        )

    def bibliography_rendered(
        self,
        *,
        style: str = "apa",
        locale: str = "en-US",
        strict: bool = False,
    ) -> pl.Expr:
        """Render a bibliography as a `{text, html}` struct."""

        return _bibliography_expr(
            "bibliography_bibtex_rendered",
            self._expr,
            style=style,
            locale=locale,
            strict=strict,
            output_name="bibliography_rendered",
        )

    def entry_count(self, *, strict: bool = False) -> pl.Expr:
        """Return the number of normalized entries in each BibTeX row."""

        return _parse_expr(
            "bibtex_entry_count",
            self._expr,
            strict=strict,
            output_name="entry_count",
        )

    def is_valid(self, *, strict: bool = False) -> pl.Expr:
        """Return whether each BibTeX row parses into a normalized library."""

        return _parse_expr("bibtex_is_valid", self._expr, strict=strict, output_name="is_valid")

    def keys(self, *, strict: bool = False) -> pl.Expr:
        """Return citation keys as a list column for each BibTeX row."""

        return _parse_expr("bibtex_keys", self._expr, strict=strict, output_name="keys")

    def entries(
        self,
        *,
        fields: Iterable[str] = ("key", "entry_type", "title", "doi", "volume"),
        strict: bool = False,
    ) -> pl.Expr:
        """Return normalized entry records as `List[Struct]` for each BibTeX row."""

        return _entries_expr(self._expr, fields=fields, strict=strict, output_name="entries")

    def parse_report(self, *, strict: bool = False) -> pl.Expr:
        """Return `{ok, entry_count, keys, diagnostics}` from one parse per row."""

        return _parse_expr(
            "bibtex_parse_report",
            self._expr,
            strict=strict,
            output_name="parse_report",
        )

    def diagnostics(self, *, strict: bool = False) -> pl.Expr:
        """Return parser diagnostics as a list column for each BibTeX row."""

        return _diagnostics_expr(self._expr, strict=strict, output_name="diagnostics")

    def entries_json(self, *, strict: bool = False) -> pl.Expr:
        """Return normalized Hayagriva entry JSON for each BibTeX row."""

        return _parse_expr(
            "bibtex_to_hayagriva_json",
            self._expr,
            strict=strict,
            output_name="entries_json",
        )

    def to_hayagriva_json(self, *, strict: bool = False) -> pl.Expr:
        """Return normalized Hayagriva entry JSON for each BibTeX row."""

        return _parse_expr(
            "bibtex_to_hayagriva_json",
            self._expr,
            strict=strict,
            output_name="to_hayagriva_json",
        )


def _cite_expr(
    function_name: str,
    bibtex: Any,
    key: Any,
    *,
    style: str,
    locale: str,
    strict: bool,
    output_name: str,
) -> pl.Expr:
    return _register(
        function_name,
        [bibtex, key],
        kwargs={"style": style, "locale": locale, "strict": strict, "all": True},
        output_name=output_name,
    )


def _cite_sequence_expr(
    function_name: str,
    bibtex: Any,
    keys: Any,
    *,
    style: str,
    locale: str,
    strict: bool,
    output_name: str,
) -> pl.Expr:
    return _register(
        function_name,
        [bibtex, keys],
        kwargs={"style": style, "locale": locale, "strict": strict, "all": True},
        output_name=output_name,
    )


def _bibliography_expr(
    function_name: str,
    bibtex: Any,
    *,
    style: str,
    locale: str,
    strict: bool,
    output_name: str,
) -> pl.Expr:
    return _register(
        function_name,
        [bibtex],
        kwargs={"style": style, "locale": locale, "strict": strict, "all": True},
        output_name=output_name,
    )


def _parse_expr(
    function_name: str,
    bibtex: Any,
    *,
    strict: bool,
    output_name: str,
) -> pl.Expr:
    return _register(
        function_name,
        [bibtex],
        kwargs={"strict": strict},
        output_name=output_name,
    )


def _entries_expr(
    bibtex: Any,
    *,
    fields: Iterable[str],
    strict: bool,
    output_name: str,
) -> pl.Expr:
    return _register(
        "bibtex_entries",
        [bibtex],
        kwargs={"strict": strict, "fields": list(fields)},
        output_name=output_name,
    )


def _diagnostics_expr(bibtex: Any, *, strict: bool, output_name: str) -> pl.Expr:
    return _register(
        "bibtex_diagnostics",
        [bibtex],
        kwargs={"strict": strict},
        output_name=output_name,
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
