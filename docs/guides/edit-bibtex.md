---
description: Apply atomic BibTeX patches with source preservation and snapshot-relative occurrence mappings in Python or TypeScript.
---

# Edit Raw BibTeX

`BibDocument` preserves source-order [BibTeX](https://ctan.org/pkg/bibtex) blocks. Apply a patch to create an edited snapshot while retaining comments, preambles, strings, malformed blocks, duplicate occurrences, delimiters, and unrelated layout.

## Update a field value

::: code-group

```python [Python]
import refkit as rk

source = "@article{doe2024, title={Fast Citations}, year={2024}}"
document = rk.BibDocument.parse(source)
title_field = document.entries["doe2024"].fields["title"]
patch_result = document.apply_patch(
    [
        {
            "kind": "set_field",
            "entry_id": title_field.entry_id,
            "field_id": title_field.id,
            "value": "Corrected title",
        }
    ]
)
document = patch_result["document"]
print(document.to_bibtex())
```

```ts [TypeScript]
import * as rk from "refkit-js";

const source = "@article{doe2024, title={Fast Citations}, year={2024}}";
let document = rk.BibDocument.parse(source);
const titleField = document.entries.getUnique("doe2024")!.fields.getUnique("title")!;
const patchResult = document.applyPatch([{
  kind: "set_field", entryId: titleField.entryId,
  fieldId: titleField.id, value: "Corrected title",
}]);
document = patchResult.document;
console.log(document.toBibtex());
```

:::

```bibtex
@article{doe2024, title={Corrected title}, year={2024}}
```

The result contains a new document, byte changes, occurrence mappings, and warnings. The original `title_field` / `titleField` still reads `Fast Citations`. Serializing the returned document preserves every byte outside its reported changes. TypeScript examples run in Node.js. In a browser, [initialize RefKit](/guides/browser#initialize-the-module) before parsing. The remaining examples continue with the returned `document`.

By default, `set_field` validates the replacement against the original delimiter mode. An unsafe replacement raises `PatchError` and leaves the input snapshot unchanged. Default values must have balanced braces and safe quote boundaries, with no newlines, unescaped percent delimiters, or trailing escape. Bare fields preserve safe bare values. Concatenated expressions become a single braced value.

Set `expression=True` / `expression: true` when `value` is one complete BibTeX expression, such as `press # { Supplement}` or `{https://example.org/a%2Fb}`. This mode preserves macros, concatenations, delimiters, and multiline braced content. Extra assignments or trailing source are rejected. It is available on `set_field`, `add_field`, and each field supplied to `add_entry`.

## Apply structural changes atomically

A patch is an ordered list of edit records. Entry and field IDs refer to occurrences in the input snapshot. Use the returned mappings to target a subsequent snapshot.

| `kind` | Required fields | Behavior |
| --- | --- | --- |
| `set_field` | Entry ID, field ID, `value` | Replace one existing raw value. |
| `add_field` | Entry ID, `name`, `value` | Append a braced field before the entry's closing delimiter. |
| `remove_field` | Entry ID, field ID | Remove the assignment and its following separator, retaining surrounding comments. |
| `add_entry` | `key`, entry type | Insert a new entry with optional ordered `fields`. Optional `before` names an input entry ID, otherwise append. |
| `remove_entry` | Entry ID | Remove one entry block. |
| `rename_entry` | Entry ID, `key` | Rename a key and rewrite unambiguous `crossref`, `xdata`, and `xref` references. |
| `set_entry_type` | Entry ID, entry type | Replace the type spelling. |

Python uses `entry_id`, `field_id`, and `entry_type`. TypeScript uses `entryId`, `fieldId`, and `entryType`. New fields use `{name, value}` records with an optional `expression` flag. New values are braced by default, and new entry/field layout uses two-space indentation. Existing unrelated layout is retained.

Multiple changes to the same existing field, key, or type are rejected. Removing an entry conflicts with edits inside it. Insertions before the same anchor retain input order and can precede an anchor removed in the same patch. Duplicate fields and keys remain representable and produce warnings in the result.

Automatic reference rewriting requires unique source and final targets. Ambiguity raises `PatchError`. Reference fields explicitly set or removed in the patch are excluded from automatic rewriting, so supply their final values. Macro definitions remain unchanged, while rewritten reference expressions become braced values.

Patches accept at most 100,000 operations and 16 MiB of authored text. Output is limited to 16 MiB. An existing larger raw snapshot can be reduced within that bound. See [patch reports](/reference/data-shapes#patch-reports) for byte changes and occurrence mappings.

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

Each call reads its document snapshot and leaves it unchanged.
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

Block kinds are `whitespace`, `comment`, `preamble`, `string`, `entry`, `failed`, and `other`. Every span is a half-open pair of UTF-8 byte offsets into that snapshot's source. Original handles retain original spans. The result document exposes updated spans.

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
second_field = second.fields["title"]
duplicate_result = duplicates.apply_patch(
    [
        {
            "kind": "set_field",
            "entry_id": second.id,
            "field_id": second_field.id,
            "value": "Revised second title",
        }
    ]
)
print(matches[0].fields["title"].value)
print(duplicate_result["document"].entries.get_all("doe2024")[1].fields["title"].value)
```

```ts [TypeScript]
const duplicates = rk.BibDocument.parse(`
@article{doe2024, title={First title}}
@article{doe2024, title={Second title}}
`);
const matches = duplicates.entries.getAll("doe2024");
const second = matches[1]!;
const secondField = second.fields.getUnique("title")!;
const duplicateResult = duplicates.applyPatch([{
  kind: "set_field", entryId: second.id,
  fieldId: secondField.id, value: "Revised second title",
}]);
console.log(matches[0]!.fields.getUnique("title")!.value);
console.log(duplicateResult.document.entries.getAll("doe2024")[1]!.fields.getUnique("title")!.value);
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
file_field = file_document.entries["doe2024"].fields["title"]
file_result = file_document.apply_patch(
    [
        {
            "kind": "set_field",
            "entry_id": file_field.entry_id,
            "field_id": file_field.id,
            "value": "Corrected title",
        }
    ]
)
file_result["document"].write("references.edited.bib")
```

```ts [TypeScript]
import { readBibDocument } from "refkit-js/node";
import { writeFile } from "node:fs/promises";

const fileDocument = await readBibDocument("references.bib");
const fileField = fileDocument.entries.getUnique("doe2024")!.fields.getUnique("title")!;
const fileResult = fileDocument.applyPatch([{
  kind: "set_field", entryId: fileField.entryId,
  fieldId: fileField.id, value: "Corrected title",
}]);
await writeFile("references.edited.bib", fileResult.document.toBibtex(), "utf8");
```

:::

File readers try UTF-8, then a Windows-1252-compatible fallback and record that choice in `diagnostics`. Original file bytes and decoded-text offsets can differ. Both examples write UTF-8.

Use [Format BibTeX](/guides/format-bibtex) for whole-document normalization according to formatting options.
