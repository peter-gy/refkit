---
description: Inspect raw BibTeX blocks and duplicate occurrences, then edit field values while preserving source layout in Python or TypeScript.
---

# Edit Raw BibTeX

`BibDocument` preserves source-order [BibTeX](https://ctan.org/pkg/bibtex) blocks while field values change. Use it when comments, preambles, strings, malformed blocks, duplicate occurrences, delimiters, or surrounding layout must remain available.

## Update a field value

::: code-group

```python [Python]
import refkit as rk

source = "@article{doe2024, title={Fast Citations}, year={2024}}"
document = rk.BibDocument.parse(source)
document.entries["doe2024"].fields["title"].value = "Corrected title"
print(document.to_bibtex())
```

```ts [TypeScript]
import * as rk from "refkit-js";

const source = "@article{doe2024, title={Fast Citations}, year={2024}}";
const document = rk.BibDocument.parse(source);
document.entries.getUnique("doe2024")!.fields.getUnique("title")!.value = "Corrected title";
console.log(document.toBibtex());
```

:::

```bibtex
@article{doe2024, title={Corrected title}, year={2024}}
```

The field handle updates the shared `BibDocument`. Serializing the document preserves the surrounding source layout. TypeScript examples run in Node.js. In a browser, [initialize RefKit](/guides/browser#initialize-the-module) before parsing. The remaining examples continue with `document`.

Field assignment validates the replacement against the original delimiter mode. An unsafe replacement raises `ValueError` in Python or `RangeError` in TypeScript before the document changes.

## Resolve fields for inspection

`resolve()` returns detached entry records with string macros and `#`
concatenations expanded in every field. Custom field names are included.
Literal TeX grouping and escapes remain available for downstream processing.

::: code-group

```python [Python]
resolved_document = rk.BibDocument.parse(r"""
@string{topic = {Visual {Data}}}
@article{guide, title = {A } # topic # { Guide \& Examples}, year = 2024}
""")
print(resolved_document.resolve()[0]["fields"]["title"])
```

```ts [TypeScript]
const resolvedDocument = rk.BibDocument.parse(String.raw`
@string{topic = {Visual {Data}}}
@article{guide, title = {A } # topic # { Guide \& Examples}, year = 2024}
`);
console.log(resolvedDocument.resolve()[0]!.fields.title);
```

:::

```text
A Visual {Data} Guide \& Examples
```

Each call uses the current field edits and leaves the document unchanged.
Entry keys retain their case. Entry types and field names are lowercase.
Macro names are case-insensitive. Definitions can appear after their uses,
and the last definition wins. Built-in month names such as `jan` expand to
`January` unless the document defines that macro.
`crossref` and `xdata` values are resolved as field text. Use `Library` when
you need inherited citation metadata and normalized names or dates.

Undefined or cyclic macros reached from entry fields, duplicate entry keys or
fields, malformed entry or string-definition syntax, and expansion-limit
violations raise `ParseError` with diagnostics. Resolution
diagnostic spans refer to the current serialized source. See
[resolved entry records](/reference/data-shapes#resolved-entry-records) for the
result shape and its Polars representation.

## Inspect source-order blocks

::: code-group

```python [Python]
for block in document.blocks:
    print(block["kind"], block["span"])
```

```ts [TypeScript]
for (const block of document.blocks) {
  console.log(block.kind, block.span);
}
```

:::

Block kinds are `whitespace`, `comment`, `preamble`, `string`, `entry`, `failed`, and `other`. Every span is a half-open pair of UTF-8 byte offsets into the decoded source text. Spans keep their original positions after edits.

Use `comments`, `preamble`, `strings`, and `failed_blocks` / `failedBlocks` for focused views. `blocks` remains the complete source-order view.

## Address duplicate entries

A key can identify several source occurrences. Parse a document with duplicate keys to inspect and update each occurrence separately:

::: code-group

```python [Python]
duplicates = rk.BibDocument.parse("""
@article{doe2024, title={First title}}
@article{doe2024, title={Second title}}
""")
matches = duplicates.entries.get_all("doe2024")
second = matches[1]
second.fields["title"].value = "Revised second title"
print(matches[0].fields["title"].value)
```

```ts [TypeScript]
const duplicates = rk.BibDocument.parse(`
@article{doe2024, title={First title}}
@article{doe2024, title={Second title}}
`);
const matches = duplicates.entries.getAll("doe2024");
const second = matches[1]!;
second.fields.getUnique("title")!.value = "Revised second title";
console.log(matches[0]!.fields.getUnique("title")!.value);
```

:::

The first occurrence still contains `First title`. The entry map exposes these lookups:

| Python | TypeScript | Result |
| --- | --- | --- |
| `unique_keys()` | `uniqueKeys()` | Each distinct key once. |
| `occurrence_keys()` | `occurrenceKeys()` | One key per source occurrence. |
| `occurrences()` | `occurrences()` | Source-order entry handles. |
| `get_all(key)` | `getAll(key)` | Every exact entry-key match. |
| `get_unique(key)` | `getUnique(key)` | One match, or `None` / `null` when missing. Duplicate matches raise `RefkitError`. |

`BibFieldMap` provides the same operations for fields. Entry keys are case-sensitive. Field-name matching is case-insensitive, and unique field keys are normalized to lowercase.

## Inspect entry and field identity

::: code-group

```python [Python]
entry = document.entries["doe2024"]
print(entry.key, entry.kind, entry.span)

field = entry.fields["title"]
print(field.name, field.value, field.span)
```

```ts [TypeScript]
const entry = document.entries.getUnique("doe2024")!;
console.log(entry.key, entry.kind, entry.span);

const field = entry.fields.getUnique("title")!;
console.log(field.name, field.value, field.span);
```

:::

`BibEntry.kind` retains the raw entry type spelling. A normalized `Library` exposes `Entry.entry_type` / `Entry.entryType`. [Bibliography Models](/concepts/bibliography-models) explains when to choose each object.

## Handle malformed blocks

::: code-group

```python [Python]
damaged = rk.BibDocument.parse(source + "\n@book{broken")
for block in damaged.failed_blocks:
    print(block["error"])
    print(block["raw"])
```

```ts [TypeScript]
const damaged = rk.BibDocument.parse(source + "\n@book{broken");
for (const block of damaged.failedBlocks) {
  console.log(block.error);
  console.log(block.raw);
}
```

:::

Malformed blocks stay in the document and its serialized output.

::: warning Percent-comment boundary
The raw parser recognizes a complete `@...` block that begins later on a percent-comment line as a live block. Inspect `BibDocument.blocks` before tidying source that embeds complete BibTeX entries inside `%` comments. Normalized report recovery suppresses those embedded blocks, while the formatter can render them as entries.
:::

## Read and write a file

Read an existing `references.bib`, update a field, and save a separate file:

::: code-group

```python [Python]
file_document = rk.BibDocument.read("references.bib")
file_document.entries["doe2024"].fields["title"].value = "Corrected title"
file_document.write("references.edited.bib")
```

```ts [TypeScript]
import { readBibDocument } from "refkit-js/node";
import { writeFile } from "node:fs/promises";

const fileDocument = await readBibDocument("references.bib");
fileDocument.entries.getUnique("doe2024")!.fields.getUnique("title")!.value = "Corrected title";
await writeFile("references.edited.bib", fileDocument.toBibtex(), "utf8");
```

:::

File readers try UTF-8, then a Windows-1252-compatible fallback and record that choice in `diagnostics`. Original file bytes and decoded-text offsets can differ. Both examples write UTF-8.

Use [Format BibTeX](/guides/format-bibtex) for whole-document normalization according to formatting options.
