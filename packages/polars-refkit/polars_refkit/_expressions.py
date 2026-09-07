from __future__ import annotations

from collections.abc import Iterable

import polars as pl

from ._plugin import (
    ColumnExpr,
    OutputFormat,
    RecoveryMode,
    _bibliography_expr,
    _cite_expr,
    _cite_list_expr,
    _diagnostics_expr,
    _entries_expr,
    _parse_expr,
    _register,
    _render_function,
    _render_kwargs,
    _tidy_expr,
)
from ._tidy_options import TidyOptions, tidy_kwargs


def cite(
    bibtex_col: ColumnExpr,
    key_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
    output: OutputFormat = "text",
) -> pl.Expr:
    """Render one key per row. Row failures return null.

    Strings name columns. Use `pl.lit(...)` for literals. Text and HTML produce
    strings, rendered produces {text, html} structs.
    """
    return _cite_expr(
        _render_function("cite", output),
        bibtex_col,
        key_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="cite",
    )


def cite_each(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
    output: OutputFormat = "text",
) -> pl.Expr:
    """Render an ordered key list with shared citation state within each row.

    Strings name columns. Use `pl.lit(...)` for literals. Text and HTML produce
    List[String], rendered produces List[Struct{text, html}]. Empty lists stay empty.
    """
    return _cite_list_expr(
        _render_function("cite_each", output),
        bibtex_col,
        keys_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="cite_each",
    )


def cite_group(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
    output: OutputFormat = "text",
) -> pl.Expr:
    """Render a nonempty key list as one grouped citation per row.

    Strings name columns. Use `pl.lit(...)` for literals. Text and HTML produce
    strings, rendered produces {text, html} structs.
    """
    return _cite_list_expr(
        _render_function("cite_group", output),
        bibtex_col,
        keys_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="cite_group",
    )


def full_bibliography(
    bibtex_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
    output: OutputFormat = "text",
) -> pl.Expr:
    """Render every entry in each BibTeX row.

    Strings name columns. Use `pl.lit(...)` for literals. Text and HTML produce
    strings, rendered produces {text, html} structs.
    """
    return _bibliography_expr(
        _render_function("full_bibliography", output),
        bibtex_col,
        style=style,
        locale=locale,
        recovery=recovery,
        output_name="full_bibliography",
    )


def render_report(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    grouped: bool = False,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr:
    """Render a key list and report citations, parse diagnostics and row errors.

    grouped=True submits one citation containing the list. Otherwise each key
    becomes one citation in an ordered sequence. Missing inputs return null.
    """
    return _register(
        "render_report",
        [bibtex_col, keys_col],
        kwargs={**_render_kwargs(style, locale, recovery), "grouped": grouped},
        output_name="render_report",
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


def tidy_bibtex(bibtex_col: ColumnExpr, *, options: TidyOptions | None = None) -> pl.Expr:
    """Format each BibTeX row and return the formatted source.

    Omitted options use core defaults. Optional rules accept True for their
    standard setting and False or None to disable them.
    """
    return _tidy_expr(
        "tidy_bibtex", bibtex_col, kwargs=tidy_kwargs(options), output_name="tidy_bibtex"
    )


def tidy_bibtex_report(bibtex_col: ColumnExpr, *, options: TidyOptions | None = None) -> pl.Expr:
    """Return {ok, bibtex, count, warnings, renames, error} per row.

    Omitted options use core defaults. Optional rules accept True for their
    standard setting and False or None to disable them.
    """
    return _tidy_expr(
        "tidy_bibtex_report",
        bibtex_col,
        kwargs=tidy_kwargs(options),
        output_name="tidy_bibtex_report",
    )
