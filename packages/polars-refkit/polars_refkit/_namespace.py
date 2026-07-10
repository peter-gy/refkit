from __future__ import annotations

from collections.abc import Iterable
from typing import Any

import polars as pl

from ._expressions import (
    can_parse,
    cite,
    cite_each,
    cite_each_html,
    cite_each_rendered,
    cite_group,
    cite_group_html,
    cite_group_rendered,
    cite_html,
    cite_rendered,
    diagnostics,
    entries,
    entry_count,
    full_bibliography_html,
    full_bibliography_rendered,
    full_bibliography_text,
    has_diagnostics,
    keys,
    parse_report,
    tidy_bibtex,
    tidy_bibtex_report,
    to_hayagriva_json,
)
from ._plugin import ColumnExpr, RecoveryMode
from ._tidy_options import TIDY_UNSET as _TIDY_UNSET


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

    def tidy_bibtex(
        self,
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

        return tidy_bibtex(
            self._expr,
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
        )

    def tidy_bibtex_report(
        self,
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

        return tidy_bibtex_report(
            self._expr,
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
        )
