from __future__ import annotations

from collections.abc import Iterable
from typing import Any

import polars as pl

from ._plugin import (
    ColumnExpr,
    RecoveryMode,
    _bibliography_expr,
    _cite_expr,
    _cite_list_expr,
    _diagnostics_expr,
    _entries_expr,
    _parse_expr,
    _tidy_expr,
)
from ._tidy_options import TIDY_UNSET as _TIDY_UNSET
from ._tidy_options import tidy_kwargs as _tidy_kwargs


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


def tidy_bibtex(
    bibtex_col: ColumnExpr,
    *,
    omit: Any = _TIDY_UNSET,
    curly: Any = _TIDY_UNSET,
    numeric: Any = _TIDY_UNSET,
    months: Any = _TIDY_UNSET,
    space: Any = _TIDY_UNSET,
    tab: Any = _TIDY_UNSET,
    align: Any = _TIDY_UNSET,
    blank_lines: Any = _TIDY_UNSET,
    sort: Any = _TIDY_UNSET,
    duplicates: Any = _TIDY_UNSET,
    merge: Any = _TIDY_UNSET,
    strip_enclosing_braces: Any = _TIDY_UNSET,
    drop_all_caps: Any = _TIDY_UNSET,
    escape: Any = _TIDY_UNSET,
    sort_fields: Any = _TIDY_UNSET,
    strip_comments: Any = _TIDY_UNSET,
    trailing_commas: Any = _TIDY_UNSET,
    encode_urls: Any = _TIDY_UNSET,
    tidy_comments: Any = _TIDY_UNSET,
    remove_empty_fields: Any = _TIDY_UNSET,
    remove_duplicate_fields: Any = _TIDY_UNSET,
    generate_keys: Any = _TIDY_UNSET,
    max_authors: Any = _TIDY_UNSET,
    lowercase: Any = _TIDY_UNSET,
    enclosing_braces: Any = _TIDY_UNSET,
    remove_braces: Any = _TIDY_UNSET,
    wrap: Any = _TIDY_UNSET,
) -> pl.Expr:
    """Format each BibTeX row and return the formatted source."""

    return _tidy_expr(
        "tidy_bibtex",
        bibtex_col,
        kwargs=_tidy_kwargs(
            omit=omit,
            curly=curly,
            numeric=numeric,
            months=months,
            space=space,
            tab=tab,
            align=align,
            blank_lines=blank_lines,
            sort=sort,
            duplicates=duplicates,
            merge=merge,
            strip_enclosing_braces=strip_enclosing_braces,
            drop_all_caps=drop_all_caps,
            escape=escape,
            sort_fields=sort_fields,
            strip_comments=strip_comments,
            trailing_commas=trailing_commas,
            encode_urls=encode_urls,
            tidy_comments=tidy_comments,
            remove_empty_fields=remove_empty_fields,
            remove_duplicate_fields=remove_duplicate_fields,
            generate_keys=generate_keys,
            max_authors=max_authors,
            lowercase=lowercase,
            enclosing_braces=enclosing_braces,
            remove_braces=remove_braces,
            wrap=wrap,
        ),
        output_name="tidy_bibtex",
    )


def tidy_bibtex_report(
    bibtex_col: ColumnExpr,
    *,
    omit: Any = _TIDY_UNSET,
    curly: Any = _TIDY_UNSET,
    numeric: Any = _TIDY_UNSET,
    months: Any = _TIDY_UNSET,
    space: Any = _TIDY_UNSET,
    tab: Any = _TIDY_UNSET,
    align: Any = _TIDY_UNSET,
    blank_lines: Any = _TIDY_UNSET,
    sort: Any = _TIDY_UNSET,
    duplicates: Any = _TIDY_UNSET,
    merge: Any = _TIDY_UNSET,
    strip_enclosing_braces: Any = _TIDY_UNSET,
    drop_all_caps: Any = _TIDY_UNSET,
    escape: Any = _TIDY_UNSET,
    sort_fields: Any = _TIDY_UNSET,
    strip_comments: Any = _TIDY_UNSET,
    trailing_commas: Any = _TIDY_UNSET,
    encode_urls: Any = _TIDY_UNSET,
    tidy_comments: Any = _TIDY_UNSET,
    remove_empty_fields: Any = _TIDY_UNSET,
    remove_duplicate_fields: Any = _TIDY_UNSET,
    generate_keys: Any = _TIDY_UNSET,
    max_authors: Any = _TIDY_UNSET,
    lowercase: Any = _TIDY_UNSET,
    enclosing_braces: Any = _TIDY_UNSET,
    remove_braces: Any = _TIDY_UNSET,
    wrap: Any = _TIDY_UNSET,
) -> pl.Expr:
    """Return `{ok, bibtex, count, warnings, error}` for each BibTeX row."""

    return _tidy_expr(
        "tidy_bibtex_report",
        bibtex_col,
        kwargs=_tidy_kwargs(
            omit=omit,
            curly=curly,
            numeric=numeric,
            months=months,
            space=space,
            tab=tab,
            align=align,
            blank_lines=blank_lines,
            sort=sort,
            duplicates=duplicates,
            merge=merge,
            strip_enclosing_braces=strip_enclosing_braces,
            drop_all_caps=drop_all_caps,
            escape=escape,
            sort_fields=sort_fields,
            strip_comments=strip_comments,
            trailing_commas=trailing_commas,
            encode_urls=encode_urls,
            tidy_comments=tidy_comments,
            remove_empty_fields=remove_empty_fields,
            remove_duplicate_fields=remove_duplicate_fields,
            generate_keys=generate_keys,
            max_authors=max_authors,
            lowercase=lowercase,
            enclosing_braces=enclosing_braces,
            remove_braces=remove_braces,
            wrap=wrap,
        ),
        output_name="tidy_bibtex_report",
    )
