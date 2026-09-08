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
| `type` | `type` | string | Entry type under this requested field name. |
| `title` | `title` | string or null | Normalized title. |
| `date` | `date` | string or null | Normalized date. |
| `doi` | `doi` | string or null | Digital object identifier. |
| `volume` | `volume` | string or null | Own volume or first parent volume. |

The default projection is `key`, `title`, `doi`, and `volume`. Both bindings return a list of `ProjectionRow` records. Each row contains exactly the requested fields, using the requested property names.

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

A `RawSpan` is a two-item Python tuple or TypeScript array, including raw blocks, entries, fields, and diagnostic locations. They index UTF-8 bytes in the original decoded source and retain those positions after edits. Python `BibDocument.write` encodes the current text as UTF-8. JavaScript `BibDocument.toBibtex()` returns the current source string for the host application to write. For a file decoded from Windows-1252, these offsets differ from the original file-byte offsets.

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
