from collections.abc import Iterable
from typing import Any

import polars as pl

__version__: str

def cite(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr: ...
def cite_bibtex(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr: ...
def cite_html(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr: ...
def cite_bibtex_html(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr: ...
def cite_rendered(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr: ...
def cite_bibtex_rendered(
    bibtex: Any,
    key: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr: ...
def cite_sequence(
    bibtex: Any,
    keys: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr: ...
def cite_bibtex_sequence(
    bibtex: Any,
    keys: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr: ...
def cite_sequence_html(
    bibtex: Any,
    keys: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr: ...
def cite_bibtex_sequence_html(
    bibtex: Any,
    keys: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr: ...
def cite_sequence_rendered(
    bibtex: Any,
    keys: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr: ...
def cite_bibtex_sequence_rendered(
    bibtex: Any,
    keys: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
) -> pl.Expr: ...
def bibliography_html(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
    all: bool = True,
) -> pl.Expr: ...
def bibliography_bibtex(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
    all: bool = True,
) -> pl.Expr: ...
def bibliography_text(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
    all: bool = True,
) -> pl.Expr: ...
def bibliography_bibtex_text(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
    all: bool = True,
) -> pl.Expr: ...
def bibliography_rendered(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
    all: bool = True,
) -> pl.Expr: ...
def bibliography_bibtex_rendered(
    bibtex: Any,
    *,
    style: str = "apa",
    locale: str = "en-US",
    strict: bool = False,
    all: bool = True,
) -> pl.Expr: ...
def entry_count(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def bibtex_entry_count(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def is_valid(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def bibtex_is_valid(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def keys(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def bibtex_keys(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def entries(
    bibtex: Any,
    *,
    fields: Iterable[str] = ("key", "entry_type", "title", "doi", "volume"),
    strict: bool = False,
) -> pl.Expr: ...
def bibtex_entries(
    bibtex: Any,
    *,
    fields: Iterable[str] = ("key", "entry_type", "title", "doi", "volume"),
    strict: bool = False,
) -> pl.Expr: ...
def diagnostics(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def bibtex_diagnostics(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def parse_report(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def bibtex_parse_report(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def bibtex_to_csl_json(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def entries_json(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def bibtex_to_hayagriva_json(bibtex: Any, *, strict: bool = False) -> pl.Expr: ...
def __getattr__(name: str) -> Any: ...
