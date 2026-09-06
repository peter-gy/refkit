---
description: Inspect RefKit projection rows, render trees, raw block records, and Polars report structs.
---

# Data Shapes

RefKit owns stable projection, render, raw-block, warning, and report shapes at its host boundaries.

## Projection rows

`Library.project` and Polars `entries` accept these fields:

| Field | Type | Meaning |
| --- | --- | --- |
| `key` | string | Citation key. |
| `entry_type` | string | Normalized entry type. |
| `type` | string | Alias of `entry_type` under the requested output name. |
| `title` | string or null | Normalized title. |
| `date` | string or null | Normalized date. |
| `doi` | string or null | Digital object identifier. |
| `volume` | string or null | Own volume or first parent volume. |

The default projection is `key`, `title`, `doi`, and `volume`.

## Render tree

`Rendered.tree` returns a list of discriminated dictionaries. The `kind` value is case-sensitive.

### `Text`

```text
{kind: "Text", text: string, formatting: Formatting}
```

### `Element`

```text
{
  kind: "Element",
  display: string | null,
  meta: string | null,
  children: RenderNode[]
}
```

`meta` names a renderer category such as `Entry`, `Name`, `Names`, `Text`, or `CitationNumber` when the renderer supplies one.

### `Markup`

```text
{kind: "Markup", value: string}
```

### `Link`

```text
{kind: "Link", text: string, url: string, formatting: Formatting}
```

### `Transparent`

```text
{kind: "Transparent", cite_idx: integer, format: string}
```

### `Formatting`

```text
{
  font_style: string,
  font_variant: string,
  font_weight: string,
  text_decoration: string,
  vertical_align: string
}
```

### Bibliography entries

Bibliography trees wrap each visible item:

```text
{
  kind: "bibliography-entry",
  key: string,
  first_field: RenderNode | null,
  children: RenderNode[]
}
```

`first_field` carries a label such as a numeric bibliography marker when the style emits one.

The current `children` list starts with the same node stored in `first_field`. A consumer that renders `children` should not render `first_field` a second time.

## Raw block records

`BibDocument.blocks` returns source-order records. Every record contains `kind` and `span`.

| Kind | Additional fields |
| --- | --- |
| `whitespace` | None. |
| `comment` | `raw`. |
| `preamble` | `value`. |
| `string` | `key`, `value`. |
| `entry` | `id`, `key`. |
| `failed` | `raw`, `error`. |
| `other` | `raw`. |

Block spans are two-item lists. `BibEntry.span` and `BibField.span` are two-item tuples. Every span is a half-open UTF-8 byte-offset pair.

## Polars parse report

```text
Struct[
  ok: Boolean,
  entry_count: UInt32,
  keys: List[String],
  diagnostics: List[String]
]
```

A parser failure returns `ok=False`, null `entry_count`, null `keys`, and diagnostic strings. A successful report returns `ok=True` and can still contain diagnostics under report recovery.

## Polars tidy report

```text
Struct[
  ok: Boolean,
  bibtex: String,
  count: UInt32,
  warnings: List[Struct[code: String, rule: String, message: String]],
  error: String
]
```

A formatting failure returns `ok=False`, null formatted fields, and an error string. A successful report returns `ok=True`, formatted source, the input entry count, and zero or more warnings.

## Polars rendered value

Rendered expressions return:

```text
Struct[text: String, html: String]
```

Polars rendered values carry text and HTML together. The Python `Rendered` object also exposes the render tree.
