from os import PathLike

from . import types as types
from ._native import (
    BibDocument,
    BibEntry,
    BibEntryMap,
    BibField,
    BibFieldMap,
    Citation,
    CitationGroup,
    Cite,
    ConversionError,
    Document,
    Library,
    Locale,
    MergeError,
    MissingReferenceError,
    ParseError,
    PatchError,
    RefkitError,
    Rendered,
    RenderedDocument,
    Style,
    TidyError,
    TidyOptions,
    TidyResult,
    TidySyntaxError,
    TidyWarning,
    build_info,
    build_mode,
    convert,
    decode,
    encode,
)
from .types import Entry as Entry

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
    "ParseError",
    "ConversionError",
    "PatchError",
    "MergeError",
    "decode",
    "encode",
    "convert",
    "Document",
    "Entry",
    "Library",
    "Locale",
    "MissingReferenceError",
    "Rendered",
    "RenderedDocument",
    "Style",
    "TidyError",
    "TidyOptions",
    "TidyResult",
    "TidySyntaxError",
    "TidyWarning",
    "build_info",
    "build_mode",
    "cite",
    "full_bibliography",
    "tidy_bibtex",
    "tidy_file",
    "__version__",
    "types",
]

__version__: str

def tidy_bibtex(
    source: str,
    *,
    options: TidyOptions | None = None,
) -> TidyResult: ...
def tidy_file(
    path: str | PathLike[str],
    *,
    output: str | PathLike[str] | None = None,
    options: TidyOptions | None = None,
) -> TidyResult: ...
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
