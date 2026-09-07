---
description: Inspect typed diagnostics, projections, rendered trees, bibliography layout, raw blocks, and Polars reports.
---

# Data Shapes

Python dictionary contracts are importable from `refkit.types`. Polars expresses the same data as scalar, list, and struct columns, with the differences named beside each shape.

## Diagnostics

`Library.diagnostics`, `BibDocument.diagnostics`, and `ParseError.diagnostics` return `list[Diagnostic]`:

| Field | Python type | Meaning |
| --- | --- | --- |
| `code` | `str` | Machine-readable diagnostic category. |
| `severity` | `"error"` or `"warning"` | Severity of this diagnostic, independent of the overall operation result. |
| `action` | `str` | `rejected`, `dropped_block`, `dropped_field`, `literalized`, or `decoded`. |
| `span` | `tuple[int, int]` or `None` | Half-open UTF-8 byte offsets into the original decoded source, when available. |
| `entry` | `str` or `None` | Affected citation key, when known. |
| `field` | `str` or `None` | Affected field, when known. |
| `message` | `str` | Human-readable explanation. |

A failed parse can contain warning diagnostics for recovery actions, such as dropped blocks, when no entries survive. Use the exception or report status to determine whether the operation succeeded.

Polars diagnostics use the same fields. `span` is a nullable `Struct[start: UInt64, end: UInt64]`. Recovery can change input length internally, but reported spans refer to the input supplied by the caller.

## Projection rows

`Library.project` and Polars `entries` accept these fields:

| Field | Value | Meaning |
| --- | --- | --- |
| `key` | string | Citation key. |
| `entry_type` | string | Normalized entry type. |
| `type` | string | Entry type under this requested field name. |
| `title` | string or null | Normalized title. |
| `date` | string or null | Normalized date. |
| `doi` | string or null | Digital object identifier. |
| `volume` | string or null | Own volume or first parent volume. |

The default projection is `key`, `title`, `doi`, and `volume`. Python returns `list[ProjectionRow]`. Each row contains the requested fields.

## Rendered nodes

`Rendered.tree` returns a fresh `RenderedTree` list. Each node has a case-sensitive `kind`:

| Kind | Fields |
| --- | --- |
| `Text` | `text: str`, `formatting: RenderedFormatting` |
| `Element` | `display: str \| None`, `meta: RenderedMeta \| None`, `children: list[RenderedNode]` |
| `Markup` | `value: str` |
| `Link` | `text: str`, `url: str`, `formatting: RenderedFormatting` |
| `Transparent` | `cite_idx: int`, `formatting: RenderedFormatting` |
| `bibliography-entry` | `key: str`, `label: RenderedNode \| None`, `content: list[RenderedNode]` |

A bibliography label and content are separate. Render the label once, followed by content. `Transparent` retains citation-index metadata and produces no visible text. Treat `Markup.value` as text when creating HTML. RefKit's HTML renderer escapes it.

### Formatting and display

`RenderedFormatting` has five required fields:

| Field | Values |
| --- | --- |
| `font_style` | `Normal`, `Italic` |
| `font_variant` | `Normal`, `SmallCaps` |
| `font_weight` | `Normal`, `Bold`, `Light` |
| `text_decoration` | `None`, `Underline` |
| `vertical_align` | `None`, `Baseline`, `Sup`, `Sub` |

`Element.display` is `Block`, `LeftMargin`, `RightInline`, `Indent`, or null. These describe layout roles rather than literal CSS values. A custom renderer chooses its corresponding elements and styles.

### Source metadata

`Element.meta` is null or a tagged dictionary:

| `kind` | Additional fields |
| --- | --- |
| `Entry` | `key: str`, `item_index: int` |
| `Names` | `roles: list[str]` |
| `Name` | `role: str`, `index: int` |
| `Date`, `Text`, `Number`, `Label`, `CitationNumber`, `CitationLabel` | None. |

The entry key identifies the bibliography record. Item and name indexes retain their positions in the rendered citation and name list.

### Bibliography layout

`Rendered.layout` is null for citation output. A bibliography returns `BibliographyLayout`:

```text
{
  hanging_indent: bool,
  second_field_align: "Margin" | "Flush" | null,
  line_spacing: int,
  entry_spacing: int
}
```

Apply these values to the bibliography as a whole. `line_spacing` describes spacing within entries and `entry_spacing` describes spacing between entries. [Render Structured Output](/guides/render-output) shows a complete tree consumer.

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

Python byte spans are two-item tuples, including raw blocks, entries, fields, and diagnostic locations. They index UTF-8 bytes in the original decoded source and retain those positions after edits. `BibDocument.write` encodes the current text as UTF-8. For a file decoded from Windows-1252, these offsets differ from the original file-byte offsets.

## Tidy renames

`TidyResult.renames` contains source-order `TidyRename` dictionaries:

```text
{entry_id: int, old_key: str, new_key: str}
```

`entry_id` identifies a source occurrence. A merged entry can map to its retained entry's final key. Use occurrence identity when duplicate source keys make a key-only map ambiguous. RefKit updates bibliography `crossref` and `xdata` references. Use the rename records to update citations in other files.

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
