from __future__ import annotations

from collections.abc import Iterable
from pathlib import Path
from typing import Any, Literal, TypeAlias

import polars as pl
from polars.plugins import register_plugin_function

PLUGIN_PATH = Path(__file__).parent
RecoveryMode: TypeAlias = Literal["error", "report"]
ColumnExpr: TypeAlias = str | pl.Expr


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


def _tidy_expr(
    function_name: str,
    bibtex_col: ColumnExpr,
    *,
    kwargs: dict[str, Any],
    output_name: str,
) -> pl.Expr:
    return _register(
        function_name,
        [bibtex_col],
        kwargs=kwargs or {"_defaults": True},
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
