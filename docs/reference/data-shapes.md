---
description: Inspect typed diagnostics, projections, rendered trees, bibliography layout, raw blocks, and Polars reports.
---

# Data Shapes

Python returns dictionaries described by [`TypedDict`](https://docs.python.org/3/library/typing.html#typing.TypedDict) definitions in `refkit.types`. TypeScript imports the corresponding object types from `refkit-js`.

::: code-group

```python [Python]
import refkit as rk
from refkit.types import ProjectionRow

library = rk.Library.parse_bibtex("@book{doe2024, title={Example}, year={2024}}")
rows: list[ProjectionRow] = library.project(["key", "entry_type"])
print(rows)  # [{'key': 'doe2024', 'entry_type': 'Book'}]
```

```ts [TypeScript]
import * as rk from "refkit-js";
import type { ProjectionRow } from "refkit-js";

const library = rk.Library.parseBibtex("@book{doe2024, title={Example}, year={2024}}");
const rows: ProjectionRow[] = library.project(["key", "entryType"]);
console.log(rows); // [{ key: "doe2024", entryType: "Book" }]
```

:::

Tables use language-neutral `string`, `integer`, `boolean`, `list`, and `null`. Python represents null as `None`. Property names match across bindings unless separate columns show a mapping. Polars expresses these data as scalar, list, and struct columns, with the differences named beside each shape.

## Conversion reports

`DecodeReport` contains `library`, `format`, and `issues`. `EncodeReport` contains `format`, `text`, and `issues`. `ConversionReport` contains source and target formats, `text`, `issues`, and parser `diagnostics`. Python uses `source_format` / `target_format`, while TypeScript uses `sourceFormat` / `targetFormat`.

| Conversion issue field | Meaning |
| --- | --- |
| `code` | Stable category such as `field_removed`, `field_changed`, `type_approximated`, or `retained_extension`. |
| `stage` | `decode` or `encode`. |
| `entry` | Affected citation key, or null when unavailable. |
| `path` | Source field or canonical record path. |
| `lossy` | Whether the issue describes data loss or a representation change refused by strict policy. |
| `message` | Explanation of the mapping or failure. |

`ConversionError` carries the same `issues` plus available parser `diagnostics`. Read [conversion policies](/guides/convert-bibliographies#inspect-conversion-loss) before accepting lossy output.

## Validation reports

`Library.validate()` and `BibDocument.validate()` return `ValidationReport` with `profile` (`records` or `biblatex`), `valid`, and `issues`. `valid` means no issue has error severity.

Each issue has `code`, `severity`, `target`, `related`, `message`, and nullable `suggestion`. Codes are `missing_required_field`, `superfluous_field`, `malformed_field`, `invalid_identifier`, `identifier_form`, `shared_identifier`, `invalid_url`, `empty_title`, `empty_name`, `reversed_date_range`, `unresolved_reference`, and `incomplete_container`.

| Target field in Python | TypeScript | Meaning |
| --- | --- | --- |
| `entry` | `entry` | Affected citation key. |
| `path` | `path` | Canonical snake_case record path, or BibLaTeX field name. |
| `entry_id` | `entryId` | Source entry occurrence index, or null. |
| `field_id` | `fieldId` | Field occurrence index within the entry, or null. |
| `span` | `span` | Half-open UTF-8 byte span in the validated snapshot, or null. |

`related` contains targets with the same shape. A missing reference has a related target naming its absent key. A shared identifier issue targets the first matching entry and lists the remaining entries in source order. Missing source fields target their entry span. Normalized records have null occurrence IDs and spans. Suggestions are strings for review and are never applied by validation.

## Diagnostics

`Library.diagnostics`, `BibDocument.diagnostics`, and `ParseError.diagnostics` return a list of `Diagnostic` records:

| Field | Value | Meaning |
| --- | --- | --- |
| `code` | string | Machine-readable diagnostic category. |
| `severity` | `"error"` or `"warning"` | Severity of this diagnostic, independent of the overall operation result. |
| `action` | string | `rejected`, `dropped_block`, `dropped_field`, `literalized`, or `decoded`. |
| `span` | byte span or null | Half-open UTF-8 byte offsets into the original decoded source, when available. |
| `entry` | string or null | Affected citation key, when known. |
| `field` | string or null | Affected field, when known. |
| `message` | string | Human-readable explanation. |

A failed parse can contain warning diagnostics for recovery actions, such as dropped blocks, when no entries survive. Use the exception or report status to determine whether the operation succeeded.

Polars diagnostics use the same fields. `span` is a nullable `Struct[start: UInt64, end: UInt64]`. Recovery can change input length internally, but reported spans refer to the input supplied by the caller.

## Projection rows

`Library.project` and Polars `entries` accept these fields:

| Python field | TypeScript field | Value | Meaning |
| --- | --- | --- | --- |
| `key` | `key` | string | Citation key. |
| `entry_type` | `entryType` | string | Normalized entry type. |
| `title` | `title` | string or null | Normalized title. |
| `date` | `date` | string or null | Normalized date. |
| `doi` | `doi` | string or null | Digital object identifier. |
| `volume` | `volume` | string or null | Own volume or first parent volume. |

The default projection is `key`, `title`, `doi`, and `volume`. Both bindings return a list of `ProjectionRow` records. Each row contains exactly the requested fields, using the requested property names.

## Bibliography records

`Library.to_records()` / `toRecords()` returns complete `Entry` values accepted by `Library.from_records()` / `fromRecords()`. Returned records are detached from the library. Omitted optional fields in constructor inputs receive the defaults shown in the types.

| Fields in Python | Fields in TypeScript | Value |
| --- | --- | --- |
| `key`, `entry_type` | `key`, `entryType` | Citation key and normalized TitleCase type. |
| `title`, `location`, `organization`, `archive`, `note`, `genre` | Same | `Text` or null. |
| `archive_location`, `call_number`, `abstract_text` | `archiveLocation`, `callNumber`, `abstractText` | `Text` or null. |
| `authors`, `editors` | Same | Ordered arrays of `Name`. |
| `affiliated` | Same | Arrays of `{role, names}` creator groups. |
| `date`, `event_date`, `original_date` | `date`, `eventDate`, `originalDate` | `BibliographyDate` or null. |
| `publisher` | Same | Optional `name` and `location` text. |
| `issue`, `chapter`, `volume`, `edition`, `runtime` | Same | `ScalarValue` or null. |
| `volume_total`, `page_range`, `page_total`, `time_range` | `volumeTotal`, `pageRange`, `pageTotal`, `timeRange` | `ScalarValue` or null. |
| `url` | Same | `{value, accessed}` URL and optional access date. |
| `identifiers` | Same | A scheme-to-string map such as `{"doi": "10.1234/work"}`. |
| `language`, `keywords` | Same | Language string or null, and an array of keywords. |
| `parents` | Same | Nested `Entry` values. Parent keys do not register separate top-level entries. |
| `extensions` | Same | Namespace-to-field maps of JSON values. Extension keys retain their spelling. |

`Text` contains `chunks` and an optional `short` array. Each chunk has `kind` (`normal`, `protected`, or `math`) and `text`. Protected chunks preserve capitalization. Math chunks retain mathematical source separately from ordinary text.

A personal `Name` has `kind: "person"`, `family`, and optional given name, prefix, suffix, alias, identifier, initials, prefix-use flag, comma-before-suffix flag, and non-dropping particle. An organizational name has `kind: "organization"` and `name`. Multiword organizations remain one name.

`BibliographyDate` contains `value`, `uncertain`, and `approximate`. Its value is a `point` with `date`, a `range` with nullable `start` and `end`, or a `literal` with `text`. Date parts contain an integer year, optional month (1–12), day (1–31, valid for its month), season (1–4), and ISO time. Years use astronomical numbering, with year zero representing 1 BCE. A season replaces the month. Time requires a complete calendar date.

`ScalarValue` has `kind: "typed"` or `"literal"` and a string `value`. The containing field determines the typed grammar: numeric values, page ranges, or durations. Volume and page totals require typed numbers. A record's volume is its own value. Scalar projection can fall back to the first parent's volume.

Record snapshots use `schema_version: 1` and snake_case fields in both bindings. Construction is bounded to 100,000 top-level records and 128 MiB of serialized record data. Nested parents and extensions are bounded to 64 levels. Bibliography source text retains its separate 16 MiB bound.

The renderer uses a date range's end, or its start when the end is open. It combines uncertainty and approximation and omits time. Literal dates, manual initials, person identifiers, prefix-use flags, keyword lists, and extension fields remain in the records even when the renderer does not use them. Malformed URLs remain in the record and are omitted from the prepared rendering view. Record snapshots preserve these values. Extension numbers must be finite, and integer values must fit JavaScript's exact integer range.

The `biblatex` extension namespace retains source-specific fields. Keys beginning with `@` describe original source type and date forms. These annotations preserve source information, while the structured fields control rendering. Editing an annotation does not change the structured field it describes.

## Resolved entry records

`BibDocument.resolve()` returns detached records in source order:

| Field | Python | TypeScript |
| --- | --- | --- |
| Citation key | `key: str` | `key: string` |
| Lowercase source entry type | `entry_type: str` | `entryType: string` |
| Expanded source fields | `fields: dict[str, str]` | `fields: Readonly<Record<string, string>>` |

Field names are lowercase. String macros and concatenations are expanded while
literal TeX grouping and escapes are preserved. The fields include custom names
and source reference keys such as `crossref`.

Resolution errors use the same diagnostic shape. Their spans address the
current serialized document, including edits made before the call.

Polars `resolve` returns `List[Struct[key: String, entry_type: String,
fields: List[Struct[name: String, value: String]]]]`. The field list represents
the dictionary within Polars' fixed dtype system. Empty bibliographies return
an empty list. Null input or a resolution failure returns null.

## Rendered nodes

`Rendered.tree` contains a `RenderedTree` list. Each node has a case-sensitive `kind`:

| Kind | Fields |
| --- | --- |
| `Text` | `text: string`, `formatting: RenderedFormatting` |
| `Element` | `display: string \| null`, `meta: RenderedMeta \| null`, `children: list[RenderedNode]` |
| `Markup` | `value: string` |
| `Link` | `text: string`, `url: string`, `formatting: RenderedFormatting` |
| `Transparent` | Integer citation index (`cite_idx` in Python, `citeIdx` in TypeScript), `formatting: RenderedFormatting`. |
| `bibliography-entry` | `key: string`, `label: RenderedNode \| null`, `content: list[RenderedNode]` |

A bibliography label and content are separate. Render the label once, followed by content. `Transparent` retains citation-index metadata and produces no visible text. Treat `Markup.value` as text when creating HTML. RefKit's HTML renderer escapes it.

### Formatting and display

`RenderedFormatting` has five required fields:

| Python field | TypeScript field | Values |
| --- | --- | --- |
| `font_style` | `fontStyle` | `Normal`, `Italic` |
| `font_variant` | `fontVariant` | `Normal`, `SmallCaps` |
| `font_weight` | `fontWeight` | `Normal`, `Bold`, `Light` |
| `text_decoration` | `textDecoration` | `None`, `Underline` |
| `vertical_align` | `verticalAlign` | `None`, `Baseline`, `Sup`, `Sub` |

`Element.display` is `Block`, `LeftMargin`, `RightInline`, `Indent`, or null. A custom renderer maps these layout roles to elements and styles.

### Source metadata

`Element.meta` is null or a record identified by its `kind`:

| `kind` | Additional fields |
| --- | --- |
| `Entry` | `key: string`, integer item index (`item_index` in Python, `itemIndex` in TypeScript) |
| `Names` | `roles: list[string]` |
| `Name` | `role: string`, `index: integer` |
| `Date`, `Text`, `Number`, `Label`, `CitationNumber`, `CitationLabel` | None. |

The entry key identifies the bibliography record. Item and name indexes retain their positions in the rendered citation and name list.

### Bibliography layout

`Rendered.layout` is null for citation output. A bibliography returns `BibliographyLayout`:

| Python field | TypeScript field | Value |
| --- | --- | --- |
| `hanging_indent` | `hangingIndent` | boolean |
| `second_field_align` | `secondFieldAlign` | `Margin`, `Flush`, or null |
| `line_spacing` | `lineSpacing` | integer |
| `entry_spacing` | `entrySpacing` | integer |

Apply these values to the bibliography as a whole. Line spacing applies within entries and entry spacing applies between entries. [Render Structured Output](/guides/render-output) shows a complete tree consumer.

## Duplicate review and merge plans

`DuplicateReport` contains the selected `rules` and candidate `groups`. Each group has `id`, `members`, `evidence`, and `conflicts`. Its ID is the first member's source entry ID. Groups and members follow source order. Evidence follows selected rule order, then signature order.

Members contain `entry_id` / `entryId` and `key`. Evidence contains `rule`, `signature`, and a list of matching member IDs. Connected matches form one candidate group.

Conflicts contain `kind` (`field`, `identifier`, or `entry_type`), `field`, and `values`. Each value contains `entry_id` / `entryId`, nullable `field_id` / `fieldId`, the inspected `value`, and its complete source `expression`. Type conflicts use field `@type` and null field IDs. Identifier conflicts distinguish different valid canonical identifier strings. Field conflicts also preserve expression-level differences for review.

`MergePlan` contains `retained_id` / `retainedId`, `removed_ids` / `removedIds`, nullable `patch`, and unresolved `conflicts`. Apply a non-null patch to the original snapshot. Field choices are `{kind: "take", name, entry_id, field_id}` in Python, using `entryId` and `fieldId` in TypeScript, or `{kind: "drop", name}` in either binding.

`MergeError.code` is `invalid_selection`, `invalid_choice`, `ambiguous_reference`, `reference_error`, `reference_cycle`, or `resource_limit`. It is distinct from an unresolved conflict report, which returns a null patch normally.

## Patch reports

`BibPatchResult` contains the new `document`, `changes`, `entries`, and `warnings`. The input snapshot remains unchanged.

Each byte change has `kind`, `operations`, `before`, and `after`. The spans index UTF-8 bytes in the input and output snapshots respectively. `operations` lists the input operation indices responsible for that change. It is empty for automatic reference rewrites. Adjacent field additions can share one byte change. Changes are ordered by input byte position, and every byte between changes is preserved.

Each entry mapping has nullable `before` and `after` `RawEntryInfo` records plus `fields`. Entry info contains `id`, `key`, `kind`, and `span`. Field mappings contain nullable `before` and `after` `RawFieldInfo` records with `id`, `name`, `value`, and `span`. A null `before` marks an addition. A null `after` marks a removal. Entry IDs are document-relative, and field IDs are relative to their entry.

Mappings list original occurrences in source order, followed by added occurrences in result order. Deleted entries include mappings for their deleted fields. Use the `after` IDs when constructing a subsequent patch against the result document.

Warnings have `code` (`duplicate_entry` or `duplicate_field`), result-snapshot `entry_id` / `entryId`, nullable `field_id` / `fieldId`, and `message`. `PatchError` has `code` and nullable `operation`. Error codes are `invalid_target`, `invalid_value`, `overlap`, `invalid_result`, `ambiguous_reference`, `reference_error`, and `resource_limit`.

## Raw blocks and spans

`BibDocument.blocks` returns source-order `RawBlock` records. Every record contains `kind` and `span`:

| Kind | Additional fields |
| --- | --- |
| `whitespace` | None. |
| `comment` | `raw`. |
| `preamble` | `value`. |
| `string` | `key`, `value`. |
| `entry` | `id`, `key`. |
| `failed` | `raw`, `error`. |
| `other` | `raw`. |

A `RawSpan` is a two-item Python tuple or TypeScript array, including raw blocks, entries, fields, and diagnostic locations. It indexes UTF-8 bytes in its document snapshot. Old handles retain old spans after a patch returns a new snapshot. Python `BibDocument.write` encodes the snapshot as UTF-8. JavaScript `BibDocument.toBibtex()` returns the source string for the host application to write. For a file decoded from Windows-1252, these offsets differ from the original file-byte offsets.

## Tidy renames

`TidyResult.renames` contains source-order `TidyRename` records:

| Python field | TypeScript field | Value |
| --- | --- | --- |
| `entry_id` | `entryId` | integer source occurrence ID |
| `old_key` | `oldKey` | original key string |
| `new_key` | `newKey` | final key string |

The entry ID identifies a source occurrence. A merged entry can map to its retained entry's final key. Use occurrence identity when duplicate source keys make a key-only map ambiguous. RefKit updates bibliography `crossref` and `xdata` references. Use the rename records to update citations in other files.

## Polars reports

Each report expression returns null for a null source row.

### Parse report

```text
Struct[
  ok: Boolean,
  entry_count: UInt32,
  keys: List[String],
  diagnostics: List[Diagnostic]
]
```

A failed parse returns `ok=False`, null `entry_count` and `keys`, and diagnostics. Report recovery can return `ok=True` with diagnostics.

### Render report

```text
Struct[
  ok: Boolean,
  citations: List[Struct[text: String, html: String]],
  diagnostics: List[Diagnostic],
  error_code: String,
  error: String
]
```

Successful output has null error fields. Failures use `parse_error`, `missing_key`, or `render_error` and an explanatory message. Null key-list input also produces a null report.

### Tidy report

```text
Struct[
  ok: Boolean,
  bibtex: String,
  count: UInt32,
  warnings: List[Struct[code: String, rule: String, message: String]],
  renames: List[Struct[entry_id: UInt64, old_key: String, new_key: String]],
  error: String
]
```

Successful output has formatted source, input entry count, warnings, rename records, and a null error. Failure has `ok=False` and an error message.
