from os import PathLike

from refkit_core import (
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
    build_info,
    build_mode,
)

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
    "build_info",
    "build_mode",
    "cite",
    "full_bibliography",
    "check_refkit_core_version",
    "__version__",
]

__version__: str

def check_refkit_core_version() -> bool: ...
def cite(
    source: str | PathLike[str],
    citation: str | Cite | CitationGroup,
    *,
    style: str | Style = "apa",
    locale: str | Locale | None = "en-US",
) -> Rendered: ...
def full_bibliography(
    source: str | PathLike[str],
    *,
    style: str | Style = "apa",
    locale: str | Locale | None = "en-US",
) -> Rendered: ...
