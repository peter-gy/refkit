"""Citation parsing, rendering, and BibTeX editing."""

from __future__ import annotations

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
    Citation,
    CitationGroup,
    Cite,
    Document,
    Entry,
    Library,
    Locale,
    MissingReferenceError,
    RefkitError,
    Rendered,
    RenderedDocument,
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
    "Citation",
    "Cite",
    "CitationGroup",
    "RefkitError",
    "Document",
    "Entry",
    "Library",
    "Locale",
    "MissingReferenceError",
    "Rendered",
    "RenderedDocument",
    "Style",
    "cite",
    "full_bibliography",
    "__version__",
]


def cite(
    source: str | PathLike[str],
    citation: str | Cite | CitationGroup,
    *,
    style: str | Style = "apa",
    locale: str | Locale | None = "en-US",
) -> Rendered:
    """Read `source` and render one citation."""

    library = Library.read(source)
    loaded_style = Style.load(style) if isinstance(style, str) else style
    rendered = Document(library, loaded_style, locale=locale).render(
        [Citation("citation", citation)]
    )
    return rendered["citation"]


def full_bibliography(
    source: str | PathLike[str],
    *,
    style: str | Style = "apa",
    locale: str | Locale | None = "en-US",
) -> Rendered:
    """Read `source` and render every entry as a bibliography."""

    library = Library.read(source)
    loaded_style = Style.load(style) if isinstance(style, str) else style
    document = Document(library, loaded_style, locale=locale)
    return document.full_bibliography()


def __getattr__(name: str) -> Any:
    raise AttributeError(f"module 'refkit' has no attribute {name!r}")
