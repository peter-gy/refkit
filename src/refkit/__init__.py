"""Citation parsing, rendering, and BibTeX editing."""

from __future__ import annotations

from collections.abc import Iterable
from importlib.metadata import PackageNotFoundError
from importlib.metadata import version as _metadata_version
from os import PathLike
from typing import Any

from ._native import (
    BibDocument,
    BibEntry,
    BibEntryMap,
    BibField,
    BibFieldMap,
    Cite,
    Document,
    Entry,
    Library,
    Locale,
    MissingReferenceError,
    RefkitError,
    Rendered,
    Style,
)
from ._native import (
    __version__ as _native_version,
)

try:
    __version__ = _metadata_version("refkit")
except PackageNotFoundError:
    __version__ = _native_version

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
    "__version__",
]


def cite(
    source: str | PathLike[str],
    item: str | Cite | Iterable[str | Cite],
    *,
    style: str | Style = "apa",
    locale: str | Locale | None = "en-US",
) -> Rendered:
    """Read `source` and render one citation group."""

    library = Library.read(source)
    loaded_style = Style.load(style) if isinstance(style, str) else style
    return Document(library, loaded_style, locale=locale).cite(item)


def bibliography(
    source: str | PathLike[str],
    *,
    style: str | Style = "apa",
    locale: str | Locale | None = "en-US",
) -> Rendered:
    """Read `source` and render every entry as a bibliography."""

    library = Library.read(source)
    loaded_style = Style.load(style) if isinstance(style, str) else style
    document = Document(library, loaded_style, locale=locale)
    return document.bibliography(all=True)


def __getattr__(name: str) -> Any:
    raise AttributeError(f"module 'refkit' has no attribute {name!r}")
