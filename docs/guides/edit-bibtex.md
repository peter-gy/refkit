---
description: Inspect raw BibTeX blocks and duplicate occurrences, then edit existing field values with preserving writeback.
---

# Edit Raw BibTeX

`BibDocument` preserves source-order blocks while field values change. Use it when comments, preambles, strings, malformed blocks, duplicate occurrences, delimiters, or surrounding layout must remain available.

## Update a Field Value

```python
import refkit as rk

document = rk.BibDocument.read("references.bib")
document.entries["doe2024"].fields["title"].value = "Corrected title"
document.write("references.bib")
assert (
    rk.BibDocument.read("references.bib").entries["doe2024"].fields["title"].value
    == "Corrected title"
)
```

The field handle updates the shared `BibDocument`. `write` serializes the current state to the selected path as UTF-8. A Windows-1252 input is decoded when read and records that choice in `document.diagnostics`. Its original file bytes and decoded-text offsets can differ.

Use `to_bibtex()` to inspect the result before writing:

```python
updated_source = document.to_bibtex()
```

## Inspect source-order blocks

```python
for block in document.blocks:
    print(block["kind"], block["span"])
```

Block kinds are `whitespace`, `comment`, `preamble`, `string`, `entry`, `failed`, and `other`. Every span is a half-open pair of UTF-8 byte offsets into the decoded source text. Spans keep their original positions after edits.

Use `comments`, `preamble`, `strings`, and `failed_blocks` for focused views. `blocks` remains the complete source-order view.

## Address duplicate entries

Direct lookup requires an unambiguous citation key:

```python
matches = document.entries.get_all("doe2024")
second = matches[1]
```

The entry map provides:

- `unique_keys()` for each distinct key name once.
- `occurrence_keys()` for one key per source occurrence.
- `occurrences()` for source-order entry handles.
- `get_all(key)` for every exact entry-key match.
- `get_unique(key)` for one match or `None` when missing. Duplicate matches raise `RefkitError`.

`BibFieldMap` provides the same operations for fields. Entry keys are case-sensitive. Field-name matching is case-insensitive, and `unique_keys()` normalizes field names to lowercase.

## Inspect entry and field identity

```python
entry = document.entries["doe2024"]
print(entry.key, entry.kind, entry.span)

field = entry.fields["title"]
print(field.name, field.value, field.span)
```

`BibEntry.kind` is the raw entry type spelling. `Entry.entry_type` is the normalized entry type in a `Library`.

## Handle malformed blocks

```python
for block in document.failed_blocks:
    print(block["error"])
    print(block["raw"])
```

Malformed blocks stay in the document and writeback unless the surrounding source is changed outside RefKit.

::: warning Percent-comment boundary
The raw parser recognizes a complete `@...` block that begins later on a percent-comment line as a live block. Inspect `BibDocument.blocks` before tidying source that embeds complete BibTeX entries inside `%` comments. Normalized report recovery suppresses those embedded blocks, while the formatter can render them as entries.
:::

## Respect the delimiter contract

Field assignment validates the replacement against the original value mode. A replacement that cannot be represented safely raises `ValueError` before the document changes.

Use [Format BibTeX](/guides/format-bibtex) when the goal is whole-document normalization. Formatting rebuilds the BibTeX layout according to `TidyOptions`.
