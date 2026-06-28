from collections.abc import Iterable
from os import PathLike
from typing import Any, Literal, TypeAlias, TypedDict

class _ProjectionRow(TypedDict, total=False):
    key: str
    entry_type: str
    type: str
    title: str | None
    doi: str | None
    volume: str | None

class _TreeFormatting(TypedDict):
    font_style: str
    font_variant: str
    font_weight: str
    text_decoration: str
    vertical_align: str

class _TreeText(TypedDict):
    kind: Literal["Text"]
    text: str
    formatting: _TreeFormatting

class _TreeElement(TypedDict):
    kind: Literal["Element"]
    display: str | None
    meta: str | None
    children: list[_TreeNode]

class _TreeMarkup(TypedDict):
    kind: Literal["Markup"]
    value: str

class _TreeLink(TypedDict):
    kind: Literal["Link"]
    text: str
    url: str
    formatting: _TreeFormatting

class _TreeTransparent(TypedDict):
    kind: Literal["Transparent"]
    cite_idx: int
    format: str

_TreeNode: TypeAlias = _TreeText | _TreeElement | _TreeMarkup | _TreeLink | _TreeTransparent

class _BibliographyEntry(TypedDict):
    kind: Literal["bibliography-entry"]
    key: str
    first_field: _TreeNode | None
    children: list[_TreeNode]

_RenderedTree: TypeAlias = list[_TreeNode | _BibliographyEntry]
_RawSpan: TypeAlias = list[int]

class _RawWhitespaceBlock(TypedDict):
    kind: Literal["whitespace"]
    span: _RawSpan

class _RawCommentBlock(TypedDict):
    kind: Literal["comment"]
    raw: str
    span: _RawSpan

class _RawPreambleBlock(TypedDict):
    kind: Literal["preamble"]
    value: str
    span: _RawSpan

class _RawStringBlock(TypedDict):
    kind: Literal["string"]
    key: str
    value: str
    span: _RawSpan

class _RawEntryBlock(TypedDict):
    kind: Literal["entry"]
    id: int
    key: str
    span: _RawSpan

class _RawFailedBlock(TypedDict):
    kind: Literal["failed"]
    raw: str
    error: str
    span: _RawSpan

class _RawOtherBlock(TypedDict):
    kind: Literal["other"]
    raw: str
    span: _RawSpan

_RawBlock: TypeAlias = (
    _RawWhitespaceBlock
    | _RawCommentBlock
    | _RawPreambleBlock
    | _RawStringBlock
    | _RawEntryBlock
    | _RawFailedBlock
    | _RawOtherBlock
)

__version__: str
build_info: str
build_mode: Literal["debug", "release"]
_tidy_option_names: list[str]
_RecoveryMode: TypeAlias = Literal["error", "report"]
_DuplicateRule: TypeAlias = Literal["doi", "key", "abstract", "citation"]
_MergeStrategy: TypeAlias = Literal["first", "last", "combine", "overwrite"]

class RefkitError(Exception): ...
class MissingReferenceError(RefkitError): ...
class TidyError(RefkitError): ...

class TidySyntaxError(TidyError):
    line: int
    column: int
    byte: int
    character: str | None
    message: str

class TidyOptions:
    def __init__(
        self,
        *,
        omit: Iterable[str] | None = None,
        curly: bool = False,
        numeric: bool = False,
        months: bool = False,
        space: int = 2,
        tab: bool = False,
        align: bool | int | None = 14,
        blank_lines: bool = False,
        sort: bool | Iterable[str] | None = None,
        duplicates: Iterable[_DuplicateRule] | None = None,
        merge: _MergeStrategy | None = None,
        strip_enclosing_braces: bool = False,
        drop_all_caps: bool = False,
        escape: bool = True,
        sort_fields: bool | Iterable[str] | None = None,
        strip_comments: bool = False,
        trailing_commas: bool = False,
        encode_urls: bool = False,
        tidy_comments: bool = True,
        remove_empty_fields: bool = False,
        remove_duplicate_fields: bool = True,
        generate_keys: bool | str | None = None,
        max_authors: int | None = None,
        lowercase: bool = True,
        enclosing_braces: bool | Iterable[str] | None = None,
        remove_braces: bool | Iterable[str] | None = None,
        wrap: bool | int | None = None,
    ) -> None: ...

class TidyWarning:
    @property
    def code(self) -> Literal["missing_key", "duplicate_entry"]: ...
    @property
    def rule(self) -> _DuplicateRule | None: ...
    @property
    def message(self) -> str: ...

class TidyResult:
    @property
    def bibtex(self) -> str: ...
    @property
    def warnings(self) -> list[TidyWarning]: ...
    @property
    def count(self) -> int: ...

def tidy_bibtex(source: str, *, options: TidyOptions | None = None) -> TidyResult: ...

class Entry:
    @property
    def key(self) -> str: ...
    @property
    def entry_type(self) -> str: ...
    @property
    def title(self) -> str | None: ...
    @property
    def parents(self) -> list[Entry]: ...
    @property
    def volume(self) -> str | None: ...
    @property
    def doi(self) -> str | None: ...

class Library:
    @staticmethod
    def read(path: str | PathLike[str], *, recovery: _RecoveryMode = "error") -> Library: ...
    @staticmethod
    def parse_bibtex(
        source: str,
        *,
        recovery: _RecoveryMode = "error",
    ) -> Library: ...
    @staticmethod
    def parse_yaml(source: str) -> Library: ...
    @property
    def diagnostics(self) -> list[str]: ...
    def keys(self) -> list[str]: ...
    def get_many(self, keys: Iterable[str]) -> list[Entry]: ...
    def values(self) -> list[Entry]: ...
    def get(self, key: str) -> Entry | None: ...
    def is_empty(self) -> bool: ...
    def select(self, selector: str) -> list[Entry]: ...
    def to_dicts(self) -> list[dict[str, Any]]: ...
    def project(
        self, fields: Iterable[str] | None = None, *, keys: Iterable[str] | None = None
    ) -> list[_ProjectionRow]: ...
    def __len__(self) -> int: ...
    def __bool__(self) -> bool: ...
    def __contains__(self, key: str) -> bool: ...
    def __getitem__(self, key: str) -> Entry: ...

class Style:
    @staticmethod
    def load(name: str) -> Style: ...
    @staticmethod
    def from_xml(xml: str) -> Style: ...
    @staticmethod
    def from_path(path: str | PathLike[str]) -> Style: ...
    @property
    def id(self) -> str: ...
    @property
    def title(self) -> str: ...

class Locale:
    @staticmethod
    def load(code: str) -> Locale: ...
    @property
    def code(self) -> str: ...

class Cite:
    def __init__(
        self, key: str, *, locator: str | None = None, label: str | None = None
    ) -> None: ...
    @property
    def key(self) -> str: ...
    @property
    def locator(self) -> str | None: ...
    @property
    def label(self) -> str | None: ...

class CitationGroup:
    def __init__(self, items: Iterable[str | Cite]) -> None: ...
    @property
    def items(self) -> list[Cite]: ...
    def __len__(self) -> int: ...

class Citation:
    def __init__(self, id: str, citation: str | Cite | CitationGroup) -> None: ...
    @property
    def id(self) -> str: ...
    @property
    def group(self) -> CitationGroup: ...

class Rendered:
    @property
    def text(self) -> str: ...
    @property
    def html(self) -> str: ...
    @property
    def tree(self) -> _RenderedTree: ...
    def to_text(self) -> str: ...
    def to_html(self) -> str: ...
    def to_tree(self) -> _RenderedTree: ...

class RenderedDocument:
    @property
    def citation_order(self) -> list[str]: ...
    @property
    def citations(self) -> dict[str, Rendered]: ...
    @property
    def bibliography(self) -> Rendered: ...
    def __getitem__(self, id: str) -> Rendered: ...

class Document:
    def __init__(
        self, library: Library, style: Style, *, locale: str | Locale | None = None
    ) -> None: ...
    def render(self, citations: Iterable[Citation]) -> RenderedDocument: ...
    def cited_bibliography(self, citations: Iterable[Citation]) -> Rendered: ...
    def full_bibliography(self) -> Rendered: ...

class BibField:
    @property
    def name(self) -> str: ...
    @property
    def value(self) -> str: ...
    @value.setter
    def value(self, value: str) -> None: ...
    @property
    def span(self) -> tuple[int, int]: ...

class BibFieldMap:
    def unique_keys(self) -> list[str]: ...
    def occurrence_keys(self) -> list[str]: ...
    def occurrences(self) -> list[BibField]: ...
    def get_all(self, key: str) -> list[BibField]: ...
    def is_empty(self) -> bool: ...
    def get_unique(self, key: str) -> BibField | None: ...
    def __len__(self) -> int: ...
    def __bool__(self) -> bool: ...
    def __contains__(self, key: str) -> bool: ...
    def __getitem__(self, key: str) -> BibField: ...

class BibEntry:
    @property
    def key(self) -> str: ...
    @property
    def kind(self) -> str: ...
    @property
    def fields(self) -> BibFieldMap: ...
    @property
    def span(self) -> tuple[int, int]: ...

class BibEntryMap:
    def unique_keys(self) -> list[str]: ...
    def occurrence_keys(self) -> list[str]: ...
    def occurrences(self) -> list[BibEntry]: ...
    def get_all(self, key: str) -> list[BibEntry]: ...
    def is_empty(self) -> bool: ...
    def get_unique(self, key: str) -> BibEntry | None: ...
    def __len__(self) -> int: ...
    def __bool__(self) -> bool: ...
    def __contains__(self, key: str) -> bool: ...
    def __getitem__(self, key: str) -> BibEntry: ...

class BibDocument:
    @staticmethod
    def read(path: str | PathLike[str]) -> BibDocument: ...
    @staticmethod
    def parse(source: str) -> BibDocument: ...
    @property
    def entries(self) -> BibEntryMap: ...
    @property
    def comments(self) -> list[str]: ...
    @property
    def preamble(self) -> str: ...
    @property
    def strings(self) -> dict[str, str]: ...
    @property
    def failed_blocks(self) -> list[_RawFailedBlock]: ...
    @property
    def blocks(self) -> list[_RawBlock]: ...
    def to_bibtex(self) -> str: ...
    def tidy(self, *, options: TidyOptions | None = None) -> TidyResult: ...
    def write(self, path: str | PathLike[str]) -> None: ...
