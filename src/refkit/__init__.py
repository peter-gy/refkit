"""Fast citation parsing, rendering, and BibTeX editing."""

from __future__ import annotations

from os import PathLike
from typing import Any

from ._native import (
    BibDocument,
    BibEntry,
    BibEntryMap,
    BibField,
    BibFieldMap,
    Cite,
    RefkitError,
    Document,
    Entry,
    Library,
    Locale,
    MissingReferenceError,
    Rendered,
    Style,
)

__all__ = [
    "BibDocument",
    "BibEntry",
    "BibEntryMap",
    "BibField",
    "BibFieldMap",
    "Cite",
    "RefkitError",
    "Document",
    "Entry",
    "Library",
    "Locale",
    "MissingReferenceError",
    "Rendered",
    "Style",
    "bibliography",
    "cite",
]


def cite(
    source: str | PathLike[str],
    item: str | Cite | list[str | Cite],
    *,
    style: str | Style = "apa",
    locale: str | Locale | None = "en-US",
) -> Rendered:
    """Render one citation from `source`."""

    library = Library.read(source)
    loaded_style = Style.load(style) if isinstance(style, str) else style
    return Document(library, loaded_style, locale=locale).cite(item)


def bibliography(
    source: str | PathLike[str],
    *,
    style: str | Style = "apa",
    locale: str | Locale | None = "en-US",
) -> Rendered:
    """Render a bibliography for every entry in `source`."""

    library = Library.read(source)
    loaded_style = Style.load(style) if isinstance(style, str) else style
    document = Document(library, loaded_style, locale=locale)
    return document.bibliography(all=True)


def __getattr__(name: str) -> Any:
    if name == "__version__":
        return "0.0.0"
    raise AttributeError(f"module 'refkit' has no attribute {name!r}")
