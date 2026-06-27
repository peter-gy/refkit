"""Polars expressions for row-level BibTeX citation workflows."""

from __future__ import annotations

from collections.abc import Iterable
from importlib.metadata import PackageNotFoundError
from importlib.metadata import version as _metadata_version
from pathlib import Path
from typing import Any, Literal, TypeAlias

import polars as pl
from polars.plugins import register_plugin_function

from ._internal import __version__ as _native_version

PLUGIN_PATH = Path(__file__).parent
RecoveryMode: TypeAlias = Literal["error", "report"]
ColumnExpr: TypeAlias = str | pl.Expr

try:
    __version__ = _metadata_version("polars-refkit")
except PackageNotFoundError:
    __version__ = _native_version

__all__ = [
    "__version__",
    "RefkitExprNamespace",
    "full_bibliography_html",
    "full_bibliography_rendered",
    "full_bibliography_text",
    "can_parse",
    "cite",
    "cite_each",
    "cite_each_html",
    "cite_each_rendered",
    "cite_group",
    "cite_group_html",
    "cite_group_rendered",
    "cite_html",
    "cite_rendered",
    "diagnostics",
    "entries",
    "entry_count",
    "has_diagnostics",
    "keys",
    "parse_report",
    "to_hayagriva_json",
]


def cite(
    bibtex_col: ColumnExpr,
    key_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Render one citation as plain text from each BibTeX row and key row."""

    return _cite_expr(
        "cite",
        bibtex_col,
        key_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="cite",
    )


def cite_html(
    bibtex_col: ColumnExpr,
    key_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Render one citation as HTML from each BibTeX row and key row."""

    return _cite_expr(
        "cite_html",
        bibtex_col,
        key_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="cite_html",
    )


def cite_rendered(
    bibtex_col: ColumnExpr,
    key_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Render one citation as a `{text, html}` struct."""

    return _cite_expr(
        "cite_rendered",
        bibtex_col,
        key_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="cite_rendered",
    )


def cite_each(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Render each key in a `List[String]` column as a separate citation."""

    return _cite_list_expr(
        "cite_each",
        bibtex_col,
        keys_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="cite_each",
    )


def cite_each_html(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Render each key in a `List[String]` column as separate citation HTML."""

    return _cite_list_expr(
        "cite_each_html",
        bibtex_col,
        keys_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="cite_each_html",
    )


def cite_each_rendered(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Render each key in a `List[String]` column as `{text, html}` structs."""

    return _cite_list_expr(
        "cite_each_rendered",
        bibtex_col,
        keys_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="cite_each_rendered",
    )


def cite_group(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Render a `List[String]` key column as one grouped citation."""

    return _cite_list_expr(
        "cite_group",
        bibtex_col,
        keys_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="cite_group",
    )


def cite_group_html(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Render a `List[String]` key column as one grouped citation in HTML."""

    return _cite_list_expr(
        "cite_group_html",
        bibtex_col,
        keys_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="cite_group_html",
    )


def cite_group_rendered(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Render a grouped citation as a `{text, html}` struct."""

    return _cite_list_expr(
        "cite_group_rendered",
        bibtex_col,
        keys_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="cite_group_rendered",
    )


def full_bibliography_html(
    bibtex_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Render every entry in each BibTeX row as an HTML bibliography."""

    return _bibliography_expr(
        "full_bibliography_html",
        bibtex_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="full_bibliography_html",
    )


def full_bibliography_text(
    bibtex_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Render every entry in each BibTeX row as a plain-text bibliography."""

    return _bibliography_expr(
        "full_bibliography_text",
        bibtex_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="full_bibliography_text",
    )


def full_bibliography_rendered(
    bibtex_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Render every entry in each BibTeX row as a `{text, html}` struct."""

    return _bibliography_expr(
        "full_bibliography_rendered",
        bibtex_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="full_bibliography_rendered",
    )


def entry_count(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr:
    """Return the number of normalized entries in each BibTeX row."""

    return _parse_expr("entry_count", bibtex_col, recovery=recovery, output_name="entry_count")


def can_parse(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr:
    """Return whether each BibTeX row parses under the selected recovery policy."""

    return _parse_expr("can_parse", bibtex_col, recovery=recovery, output_name="can_parse")


def has_diagnostics(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr:
    """Return whether parser diagnostics were reported for each BibTeX row."""

    return _parse_expr(
        "has_diagnostics",
        bibtex_col,
        recovery=recovery,
        output_name="has_diagnostics",
    )


def keys(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr:
    """Return citation keys as a list column for each BibTeX row."""

    return _parse_expr("keys", bibtex_col, recovery=recovery, output_name="keys")


def entries(
    bibtex_col: ColumnExpr,
    *,
    fields: Iterable[str] | None = None,
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Return normalized entry records as `List[Struct]` for each BibTeX row."""

    if isinstance(fields, str):
        raise TypeError("fields must be an iterable of field names")
    selected_fields = ("key", "title", "doi", "volume") if fields is None else tuple(fields)
    return _entries_expr(
        bibtex_col,
        fields=selected_fields,
        recovery=recovery,
        output_name="entries",
    )


def diagnostics(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr:
    """Return parser diagnostics as a list column for each BibTeX row."""

    return _diagnostics_expr(bibtex_col, recovery=recovery, output_name="diagnostics")


def parse_report(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr:
    """Return `{ok, entry_count, keys, diagnostics}` from one parse per row."""

    return _parse_expr("parse_report", bibtex_col, recovery=recovery, output_name="parse_report")


def to_hayagriva_json(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr:
    """Return normalized Hayagriva entry JSON for each BibTeX row."""

    return _parse_expr(
        "to_hayagriva_json",
        bibtex_col,
        recovery=recovery,
        output_name="to_hayagriva_json",
    )


@pl.api.register_expr_namespace("refkit")
class RefkitExprNamespace:
    """BibTeX expressions available from `pl.Expr.refkit`."""

    def __init__(self, expr: pl.Expr) -> None:
        self._expr = expr

    def cite(
        self,
        key_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        """Render one citation from each BibTeX row and key row."""

        return cite(self._expr, key_col, style=style, locale=locale, recovery=recovery)

    def cite_html(
        self,
        key_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        """Render one citation as HTML from each BibTeX row and key row."""

        return cite_html(self._expr, key_col, style=style, locale=locale, recovery=recovery)

    def cite_rendered(
        self,
        key_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        """Render one citation as a `{text, html}` struct."""

        return cite_rendered(self._expr, key_col, style=style, locale=locale, recovery=recovery)

    def cite_each(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        """Render each key in a `List[String]` column as a separate citation."""

        return cite_each(self._expr, keys_col, style=style, locale=locale, recovery=recovery)

    def cite_each_html(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        """Render each key in a `List[String]` column as separate citation HTML."""

        return cite_each_html(self._expr, keys_col, style=style, locale=locale, recovery=recovery)

    def cite_each_rendered(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        """Render each key in a `List[String]` column as `{text, html}` structs."""

        return cite_each_rendered(
            self._expr,
            keys_col,
            style=style,
            locale=locale,
            recovery=recovery,
        )

    def cite_group(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        """Render a `List[String]` key column as one grouped citation."""

        return cite_group(self._expr, keys_col, style=style, locale=locale, recovery=recovery)

    def cite_group_html(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        """Render a `List[String]` key column as one grouped citation in HTML."""

        return cite_group_html(self._expr, keys_col, style=style, locale=locale, recovery=recovery)

    def cite_group_rendered(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        """Render a grouped citation as a `{text, html}` struct."""

        return cite_group_rendered(
            self._expr,
            keys_col,
            style=style,
            locale=locale,
            recovery=recovery,
        )

    def full_bibliography_html(
        self,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        """Render every entry in each BibTeX row as an HTML bibliography."""

        return full_bibliography_html(self._expr, style=style, locale=locale, recovery=recovery)

    def full_bibliography_text(
        self,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        """Render every entry in each BibTeX row as a plain-text bibliography."""

        return full_bibliography_text(self._expr, style=style, locale=locale, recovery=recovery)

    def full_bibliography_rendered(
        self,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        """Render every entry in each BibTeX row as a `{text, html}` struct."""

        return full_bibliography_rendered(self._expr, style=style, locale=locale, recovery=recovery)

    def entry_count(self, *, recovery: RecoveryMode = "error") -> pl.Expr:
        """Return the number of normalized entries in each BibTeX row."""

        return entry_count(self._expr, recovery=recovery)

    def can_parse(self, *, recovery: RecoveryMode = "error") -> pl.Expr:
        """Return whether each BibTeX row parses under the selected recovery policy."""

        return can_parse(self._expr, recovery=recovery)

    def has_diagnostics(self, *, recovery: RecoveryMode = "error") -> pl.Expr:
        """Return whether parser diagnostics were reported for each BibTeX row."""

        return has_diagnostics(self._expr, recovery=recovery)

    def keys(self, *, recovery: RecoveryMode = "error") -> pl.Expr:
        """Return citation keys as a list column for each BibTeX row."""

        return keys(self._expr, recovery=recovery)

    def entries(
        self,
        *,
        fields: Iterable[str] | None = None,
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        """Return normalized entry records as `List[Struct]` for each BibTeX row."""

        return entries(self._expr, fields=fields, recovery=recovery)

    def parse_report(self, *, recovery: RecoveryMode = "error") -> pl.Expr:
        """Return `{ok, entry_count, keys, diagnostics}` from one parse per row."""

        return parse_report(self._expr, recovery=recovery)

    def diagnostics(self, *, recovery: RecoveryMode = "error") -> pl.Expr:
        """Return parser diagnostics as a list column for each BibTeX row."""

        return diagnostics(self._expr, recovery=recovery)

    def to_hayagriva_json(self, *, recovery: RecoveryMode = "error") -> pl.Expr:
        """Return normalized Hayagriva entry JSON for each BibTeX row."""

        return to_hayagriva_json(self._expr, recovery=recovery)


def _cite_expr(
    function_name: str,
    bibtex_col: ColumnExpr,
    key_col: ColumnExpr,
    *,
    style: str,
    locale: str,
    recovery: RecoveryMode,
    output_name: str,
) -> pl.Expr:
    return _register(
        function_name,
        [bibtex_col, key_col],
        kwargs=_render_kwargs(style, locale, recovery),
        output_name=output_name,
    )


def _cite_list_expr(
    function_name: str,
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str,
    locale: str,
    recovery: RecoveryMode,
    output_name: str,
) -> pl.Expr:
    return _register(
        function_name,
        [bibtex_col, keys_col],
        kwargs=_render_kwargs(style, locale, recovery),
        output_name=output_name,
    )


def _bibliography_expr(
    function_name: str,
    bibtex_col: ColumnExpr,
    *,
    style: str,
    locale: str,
    recovery: RecoveryMode,
    output_name: str,
) -> pl.Expr:
    return _register(
        function_name,
        [bibtex_col],
        kwargs=_render_kwargs(style, locale, recovery),
        output_name=output_name,
    )


def _parse_expr(
    function_name: str,
    bibtex_col: ColumnExpr,
    *,
    recovery: RecoveryMode,
    output_name: str,
) -> pl.Expr:
    return _register(
        function_name,
        [bibtex_col],
        kwargs={"strict": _recovery_to_strict(recovery)},
        output_name=output_name,
    )


def _entries_expr(
    bibtex_col: ColumnExpr,
    *,
    fields: Iterable[str],
    recovery: RecoveryMode,
    output_name: str,
) -> pl.Expr:
    return _register(
        "entries",
        [bibtex_col],
        kwargs={"strict": _recovery_to_strict(recovery), "fields": list(fields)},
        output_name=output_name,
    )


def _diagnostics_expr(
    bibtex_col: ColumnExpr, *, recovery: RecoveryMode, output_name: str
) -> pl.Expr:
    return _register(
        "diagnostics",
        [bibtex_col],
        kwargs={"strict": _recovery_to_strict(recovery)},
        output_name=output_name,
    )


def _render_kwargs(style: str, locale: str, recovery: RecoveryMode) -> dict[str, Any]:
    return {
        "style": style,
        "locale": locale,
        "strict": _recovery_to_strict(recovery),
        "all": True,
    }


def _recovery_to_strict(recovery: RecoveryMode) -> bool:
    if recovery == "error":
        return True
    if recovery == "report":
        return False
    raise ValueError("recovery must be 'error' or 'report'")


def _register(
    function_name: str,
    args: list[ColumnExpr],
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


def _parse_into_expr(expr: ColumnExpr) -> pl.Expr:
    if isinstance(expr, pl.Expr):
        return expr
    if isinstance(expr, str):
        return pl.col(expr)
    return pl.lit(expr)


def __getattr__(name: str) -> Any:
    raise AttributeError(f"module 'polars_refkit' has no attribute {name!r}")
