from typing import Any

import polars as pl

__version__: str

def cite_bibtex(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr: ...
def bibliography_bibtex(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    all: bool = True,
    strict: bool = False,
) -> pl.Expr: ...
def bibtex_entry_count(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def bibtex_keys(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def bibtex_diagnostics(bibtex: Any) -> pl.Expr: ...
def bibtex_to_csl_json(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def __getattr__(name: str) -> Any: ...
