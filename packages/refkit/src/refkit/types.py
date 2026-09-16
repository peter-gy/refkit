"""Typed records returned by RefKit parsing, editing, and rendering APIs."""

from __future__ import annotations

from typing import Literal, TypeAlias, TypedDict

from ._native import BibDocument, Library

BibliographyFormat: TypeAlias = Literal["biblatex", "hayagriva", "csl-json"]
LossPolicy: TypeAlias = Literal["error", "report"]


class ConversionIssue(TypedDict):
    code: str
    stage: Literal["decode", "encode"]
    entry: str | None
    path: str
    lossy: bool
    message: str


class DecodeReport(TypedDict):
    library: Library
    format: BibliographyFormat
    issues: list[ConversionIssue]


class EncodeReport(TypedDict):
    format: BibliographyFormat
    text: str
    issues: list[ConversionIssue]


class ConversionReport(TypedDict):
    source_format: BibliographyFormat
    target_format: BibliographyFormat
    text: str
    issues: list[ConversionIssue]
    diagnostics: list[Diagnostic]


class TextChunk(TypedDict):
    kind: Literal["normal", "protected", "math"]
    text: str


class _TextChunks(TypedDict):
    chunks: list[TextChunk]


class Text(_TextChunks, total=False):
    short: list[TextChunk] | None


class _PersonIdentity(TypedDict):
    kind: Literal["person"]
    family: str


class PersonalName(_PersonIdentity, total=False):
    given: str | None
    prefix: str | None
    suffix: str | None
    alias: str | None
    comma_suffix: bool
    id: str | None
    given_initials: str | None
    prefix_initials: str | None
    use_prefix: bool | None
    non_dropping_particle: str | None


class OrganizationName(TypedDict):
    kind: Literal["organization"]
    name: str


Name: TypeAlias = PersonalName | OrganizationName


class Contributors(TypedDict):
    role: str
    names: list[Name]


class _DateYear(TypedDict):
    year: int


class DateParts(_DateYear, total=False):
    month: int | None
    day: int | None
    season: int | None
    time: str | None


class PointDate(TypedDict):
    kind: Literal["point"]
    date: DateParts


class RangeDate(TypedDict):
    kind: Literal["range"]
    start: DateParts | None
    end: DateParts | None


class LiteralDate(TypedDict):
    kind: Literal["literal"]
    text: str


DateValue: TypeAlias = PointDate | RangeDate | LiteralDate


class _DateValue(TypedDict):
    value: DateValue


class BibliographyDate(_DateValue, total=False):
    uncertain: bool
    approximate: bool


class ScalarValue(TypedDict):
    kind: Literal["typed", "literal"]
    value: str


class Publisher(TypedDict, total=False):
    name: Text | None
    location: Text | None


class _UrlValue(TypedDict):
    value: str


class BibliographyUrl(_UrlValue, total=False):
    accessed: BibliographyDate | None


ExtensionValue: TypeAlias = (
    bool | float | str | None | list["ExtensionValue"] | dict[str, "ExtensionValue"]
)


class _EntryIdentity(TypedDict):
    key: str
    entry_type: str


class Entry(_EntryIdentity, total=False):
    title: Text | None
    authors: list[Name]
    editors: list[Name]
    affiliated: list[Contributors]
    date: BibliographyDate | None
    event_date: BibliographyDate | None
    original_date: BibliographyDate | None
    publisher: Publisher | None
    location: Text | None
    organization: Text | None
    issue: ScalarValue | None
    chapter: ScalarValue | None
    volume: ScalarValue | None
    volume_total: ScalarValue | None
    edition: ScalarValue | None
    page_range: ScalarValue | None
    page_total: ScalarValue | None
    time_range: ScalarValue | None
    runtime: ScalarValue | None
    url: BibliographyUrl | None
    identifiers: dict[str, str]
    language: str | None
    archive: Text | None
    archive_location: Text | None
    call_number: Text | None
    note: Text | None
    abstract_text: Text | None
    genre: Text | None
    keywords: list[str]
    parents: list[Entry]
    extensions: dict[str, dict[str, ExtensionValue]]


CitePurpose: TypeAlias = Literal["normal", "author", "year", "full", "prose"]


class StyleMetadata(TypedDict):
    name: str
    aliases: list[str]
    title: str
    csl_id: str


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
    title: str | None
    date: str | None
    doi: str | None
    volume: str | None


class ResolvedBibEntry(TypedDict):
    key: str
    entry_type: str
    fields: dict[str, str]


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


class RawEntryInfo(TypedDict):
    id: int
    key: str
    kind: str
    span: RawSpan


class RawFieldInfo(TypedDict):
    id: int
    name: str
    value: str
    span: RawSpan


class _ExpressionOption(TypedDict, total=False):
    expression: bool


class BibFieldValue(_ExpressionOption):
    name: str
    value: str


class SetField(_ExpressionOption):
    kind: Literal["set_field"]
    entry_id: int
    field_id: int
    value: str


class AddField(_ExpressionOption):
    kind: Literal["add_field"]
    entry_id: int
    name: str
    value: str


class RemoveField(TypedDict):
    kind: Literal["remove_field"]
    entry_id: int
    field_id: int


class _AddEntryIdentity(TypedDict):
    kind: Literal["add_entry"]
    key: str
    entry_type: str


class AddEntry(_AddEntryIdentity, total=False):
    fields: list[BibFieldValue]
    before: int | None


class RemoveEntry(TypedDict):
    kind: Literal["remove_entry"]
    entry_id: int


class RenameEntry(TypedDict):
    kind: Literal["rename_entry"]
    entry_id: int
    key: str


class SetEntryType(TypedDict):
    kind: Literal["set_entry_type"]
    entry_id: int
    entry_type: str


BibEdit: TypeAlias = (
    SetField | AddField | RemoveField | AddEntry | RemoveEntry | RenameEntry | SetEntryType
)
BibPatch: TypeAlias = list[BibEdit]
BibPatchErrorCode: TypeAlias = Literal[
    "invalid_target",
    "invalid_value",
    "overlap",
    "invalid_result",
    "ambiguous_reference",
    "reference_error",
    "resource_limit",
]


class BibPatchChange(TypedDict):
    operations: list[int]
    kind: Literal[
        "set_field",
        "add_field",
        "remove_field",
        "add_entry",
        "remove_entry",
        "rename_entry",
        "set_entry_type",
        "rewrite_reference",
    ]
    before: RawSpan
    after: RawSpan


class BibFieldMapping(TypedDict):
    before: RawFieldInfo | None
    after: RawFieldInfo | None


class BibEntryMapping(TypedDict):
    before: RawEntryInfo | None
    after: RawEntryInfo | None
    fields: list[BibFieldMapping]


class BibPatchWarning(TypedDict):
    code: Literal["duplicate_entry", "duplicate_field"]
    entry_id: int
    field_id: int | None
    message: str


class BibPatchResult(TypedDict):
    document: BibDocument
    changes: list[BibPatchChange]
    entries: list[BibEntryMapping]
    warnings: list[BibPatchWarning]


DuplicateRule: TypeAlias = Literal["doi", "key", "abstract", "citation"]


class DuplicateMember(TypedDict):
    entry_id: int
    key: str


class DuplicateEvidence(TypedDict):
    rule: DuplicateRule
    signature: str
    members: list[int]


class DuplicateValue(TypedDict):
    entry_id: int
    field_id: int | None
    value: str
    expression: str


class DuplicateConflict(TypedDict):
    kind: Literal["field", "identifier", "entry_type"]
    field: str
    values: list[DuplicateValue]


class DuplicateGroup(TypedDict):
    id: int
    members: list[DuplicateMember]
    evidence: list[DuplicateEvidence]
    conflicts: list[DuplicateConflict]


class DuplicateReport(TypedDict):
    rules: list[DuplicateRule]
    groups: list[DuplicateGroup]


class TakeMergeField(TypedDict):
    kind: Literal["take"]
    name: str
    entry_id: int
    field_id: int


class DropMergeField(TypedDict):
    kind: Literal["drop"]
    name: str


MergeFieldChoice: TypeAlias = TakeMergeField | DropMergeField
MergeErrorCode: TypeAlias = Literal[
    "invalid_selection",
    "invalid_choice",
    "ambiguous_reference",
    "reference_error",
    "reference_cycle",
    "resource_limit",
]


class MergePlan(TypedDict):
    retained_id: int
    removed_ids: list[int]
    patch: BibPatch | None
    conflicts: list[DuplicateConflict]


ValidationCode: TypeAlias = Literal[
    "missing_required_field",
    "superfluous_field",
    "malformed_field",
    "invalid_identifier",
    "identifier_form",
    "shared_identifier",
    "invalid_url",
    "empty_title",
    "empty_name",
    "reversed_date_range",
    "unresolved_reference",
    "incomplete_container",
]


class ValidationTarget(TypedDict):
    entry: str
    path: str
    entry_id: int | None
    field_id: int | None
    span: RawSpan | None


class ValidationIssue(TypedDict):
    code: ValidationCode
    severity: Literal["error", "warning"]
    target: ValidationTarget
    related: list[ValidationTarget]
    message: str
    suggestion: str | None


class ValidationReport(TypedDict):
    profile: Literal["records", "biblatex"]
    valid: bool
    issues: list[ValidationIssue]


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
