"""Citation parsing, rendering, and BibTeX editing."""

from __future__ import annotations

from importlib.metadata import version as _metadata_version
from os import PathLike
from pathlib import Path

import refkit_core as _core

__version__ = _metadata_version("refkit")


def check_refkit_core_version() -> bool:
    """Return whether the installed `refkit-core` version matches `refkit`."""

    return _core.__version__ == __version__


def _ensure_refkit_core_version() -> None:
    if check_refkit_core_version():
        return
    raise SystemError(
        f"The installed refkit-core version ({_core.__version__}) is incompatible "
        f"with refkit {__version__}. "
        "Install refkit and refkit-core from the same release."
    )


_ensure_refkit_core_version()

BibDocument = _core.BibDocument
BibEntry = _core.BibEntry
BibEntryMap = _core.BibEntryMap
BibField = _core.BibField
BibFieldMap = _core.BibFieldMap
Citation = _core.Citation
CitationGroup = _core.CitationGroup
Cite = _core.Cite
Document = _core.Document
Entry = _core.Entry
Library = _core.Library
Locale = _core.Locale
MissingReferenceError = _core.MissingReferenceError
RefkitError = _core.RefkitError
Rendered = _core.Rendered
RenderedDocument = _core.RenderedDocument
Style = _core.Style
TidyError = _core.TidyError
TidyOptions = _core.TidyOptions
TidyResult = _core.TidyResult
TidySyntaxError = _core.TidySyntaxError
TidyWarning = _core.TidyWarning
build_info = _core.build_info
build_mode = _core.build_mode

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
    "check_refkit_core_version",
    "__version__",
]


def tidy_bibtex(
    source: str,
    *,
    options: TidyOptions | None = None,
) -> TidyResult:
    """Format BibTeX text and return the formatted source plus warnings."""

    return _core.tidy_bibtex(source, options=options)


def tidy_file(
    path: str | PathLike[str],
    *,
    output: str | PathLike[str] | None = None,
    options: TidyOptions | None = None,
) -> TidyResult:
    """Read a BibTeX file, format it, and write the result when `output` is set."""

    result = BibDocument.read(path).tidy(options=options)
    if output is not None:
        Path(output).write_text(result.bibtex, encoding="utf-8")
    return result


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
