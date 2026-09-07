"""Typed records returned by RefKit parsing, editing, and rendering APIs."""

from __future__ import annotations

from typing import Literal, TypeAlias, TypedDict


class EntryMeta(TypedDict):
    kind: Literal["Entry"]
    key: str
    item_index: int


class NamesMeta(TypedDict):
    kind: Literal["Names"]
    roles: list[str]


class NameMeta(TypedDict):
    kind: Literal["Name"]
    role: str
    index: int


class ValueMeta(TypedDict):
    kind: Literal["Date", "Text", "Number", "Label", "CitationNumber", "CitationLabel"]


RenderedMeta: TypeAlias = EntryMeta | NamesMeta | NameMeta | ValueMeta


class BibliographyLayout(TypedDict):
    hanging_indent: bool
    second_field_align: Literal["Margin", "Flush"] | None
    line_spacing: int
    entry_spacing: int


class Diagnostic(TypedDict):
    code: str
    severity: Literal["error", "warning"]
    action: Literal["rejected", "dropped_block", "dropped_field", "literalized", "decoded"]
    span: tuple[int, int] | None
    entry: str | None
    field: str | None
    message: str


class TidyRename(TypedDict):
    entry_id: int
    old_key: str
    new_key: str


class ProjectionRow(TypedDict, total=False):
    key: str
    entry_type: str
    type: str
    title: str | None
    date: str | None
    doi: str | None
    volume: str | None


class RenderedFormatting(TypedDict):
    font_style: Literal["Normal", "Italic"]
    font_variant: Literal["Normal", "SmallCaps"]
    font_weight: Literal["Normal", "Bold", "Light"]
    text_decoration: Literal["None", "Underline"]
    vertical_align: Literal["None", "Baseline", "Sup", "Sub"]


class RenderedText(TypedDict):
    kind: Literal["Text"]
    text: str
    formatting: RenderedFormatting


class RenderedElement(TypedDict):
    kind: Literal["Element"]
    display: Literal["Block", "LeftMargin", "RightInline", "Indent"] | None
    meta: RenderedMeta | None
    children: list[RenderedNode]


class RenderedMarkup(TypedDict):
    kind: Literal["Markup"]
    value: str


class RenderedLink(TypedDict):
    kind: Literal["Link"]
    text: str
    url: str
    formatting: RenderedFormatting


class RenderedTransparent(TypedDict):
    kind: Literal["Transparent"]
    cite_idx: int
    formatting: RenderedFormatting


RenderedNode: TypeAlias = (
    RenderedText | RenderedElement | RenderedMarkup | RenderedLink | RenderedTransparent
)


class BibliographyEntry(TypedDict):
    kind: Literal["bibliography-entry"]
    key: str
    label: RenderedNode | None
    content: list[RenderedNode]


RenderedTree: TypeAlias = list[RenderedNode | BibliographyEntry]
RawSpan: TypeAlias = tuple[int, int]


class RawWhitespaceBlock(TypedDict):
    kind: Literal["whitespace"]
    span: RawSpan


class RawCommentBlock(TypedDict):
    kind: Literal["comment"]
    raw: str
    span: RawSpan


class RawPreambleBlock(TypedDict):
    kind: Literal["preamble"]
    value: str
    span: RawSpan


class RawStringBlock(TypedDict):
    kind: Literal["string"]
    key: str
    value: str
    span: RawSpan


class RawEntryBlock(TypedDict):
    kind: Literal["entry"]
    id: int
    key: str
    span: RawSpan


class RawFailedBlock(TypedDict):
    kind: Literal["failed"]
    raw: str
    error: str
    span: RawSpan


class RawOtherBlock(TypedDict):
    kind: Literal["other"]
    raw: str
    span: RawSpan


RawBlock: TypeAlias = (
    RawWhitespaceBlock
    | RawCommentBlock
    | RawPreambleBlock
    | RawStringBlock
    | RawEntryBlock
    | RawFailedBlock
    | RawOtherBlock
)
