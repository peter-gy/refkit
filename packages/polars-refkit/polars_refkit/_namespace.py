from __future__ import annotations

from collections.abc import Iterable

import polars as pl

from . import _expressions as expressions
from ._plugin import ColumnExpr, OutputFormat, RecoveryMode
from ._tidy_options import TidyOptions


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
        output: OutputFormat = "text",
    ) -> pl.Expr:
        return expressions.cite(
            self._expr, key_col, style=style, locale=locale, recovery=recovery, output=output
        )

    def cite_each(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
        output: OutputFormat = "text",
    ) -> pl.Expr:
        return expressions.cite_each(
            self._expr, keys_col, style=style, locale=locale, recovery=recovery, output=output
        )

    def cite_group(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
        output: OutputFormat = "text",
    ) -> pl.Expr:
        return expressions.cite_group(
            self._expr, keys_col, style=style, locale=locale, recovery=recovery, output=output
        )

    def full_bibliography(
        self,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
        output: OutputFormat = "text",
    ) -> pl.Expr:
        return expressions.full_bibliography(
            self._expr, style=style, locale=locale, recovery=recovery, output=output
        )

    def render_report(
        self,
        keys_col: ColumnExpr,
        *,
        grouped: bool = False,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr:
        return expressions.render_report(
            self._expr, keys_col, grouped=grouped, style=style, locale=locale, recovery=recovery
        )

    def entry_count(self, *, recovery: RecoveryMode = "error") -> pl.Expr:
        return expressions.entry_count(self._expr, recovery=recovery)

    def can_parse(self, *, recovery: RecoveryMode = "error") -> pl.Expr:
        return expressions.can_parse(self._expr, recovery=recovery)

    def has_diagnostics(self, *, recovery: RecoveryMode = "error") -> pl.Expr:
        return expressions.has_diagnostics(self._expr, recovery=recovery)

    def keys(self, *, recovery: RecoveryMode = "error") -> pl.Expr:
        return expressions.keys(self._expr, recovery=recovery)

    def entries(
        self, *, fields: Iterable[str] | None = None, recovery: RecoveryMode = "error"
    ) -> pl.Expr:
        return expressions.entries(self._expr, fields=fields, recovery=recovery)

    def diagnostics(self, *, recovery: RecoveryMode = "error") -> pl.Expr:
        return expressions.diagnostics(self._expr, recovery=recovery)

    def resolve(self) -> pl.Expr:
        return expressions.resolve(self._expr)

    def parse_report(self, *, recovery: RecoveryMode = "error") -> pl.Expr:
        return expressions.parse_report(self._expr, recovery=recovery)

    def tidy_bibtex(self, *, options: TidyOptions | None = None) -> pl.Expr:
        return expressions.tidy_bibtex(self._expr, options=options)

    def tidy_bibtex_report(self, *, options: TidyOptions | None = None) -> pl.Expr:
        return expressions.tidy_bibtex_report(self._expr, options=options)
