"""Native core for refkit."""

from __future__ import annotations

from importlib.metadata import PackageNotFoundError
from importlib.metadata import version as _metadata_version

from . import _refkit_core as _extension

try:
    __version__ = _metadata_version("refkit-core")
except PackageNotFoundError:
    __version__ = _extension.__version__

if __version__ != _extension.__version__:
    raise SystemError(
        f"The installed refkit-core extension version ({_extension.__version__}) "
        f"is incompatible with refkit-core {__version__}. "
        "Install refkit-core from one release."
    )

BibDocument = _extension.BibDocument
BibEntry = _extension.BibEntry
BibEntryMap = _extension.BibEntryMap
BibField = _extension.BibField
BibFieldMap = _extension.BibFieldMap
Citation = _extension.Citation
CitationGroup = _extension.CitationGroup
Cite = _extension.Cite
Document = _extension.Document
Entry = _extension.Entry
Library = _extension.Library
Locale = _extension.Locale
MissingReferenceError = _extension.MissingReferenceError
RefkitError = _extension.RefkitError
Rendered = _extension.Rendered
RenderedDocument = _extension.RenderedDocument
Style = _extension.Style
build_info = _extension.build_info
build_mode = _extension.build_mode

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
    "__version__",
]
