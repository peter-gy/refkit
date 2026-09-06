---
description: Choose Library for normalized behavior or BibDocument for source-preserving BibTeX edits.
---

# Choose a Bibliography Model

`Library` answers questions about bibliography meaning. `BibDocument` answers questions about raw BibTeX source. Choose the model from the information the workflow must retain.

## Use `Library` for normalized data

```python
import refkit as rk

library = rk.Library.parse_bibtex(
    "@article{doe2024, title={Fast Citations}, year={2024}}"
)

entry = library["doe2024"]
print(entry.key, entry.entry_type, entry.title)
```

`Library` owns:

- Normalized entries and parent relationships.
- Key lookup and ordered entry access.
- Hayagriva selectors.
- Projection into RefKit-owned dictionary fields.
- Parser diagnostics from report recovery.
- Input to citation and bibliography rendering.

Normalized records discard raw layout details that have no normalized citation meaning.

## Use `BibDocument` for source-preserving edits

```python
import refkit as rk

source = """% reviewed by Jane
@article{doe2024,
  title = {Old title},
  year = {2024}
}
"""

document = rk.BibDocument.parse(source)
document.entries["doe2024"].fields["title"].value = "Corrected title"

print(document.to_bibtex())
```

`BibDocument` owns:

- Whitespace, comments, preambles, string definitions, entries, failed blocks, and other top-level source.
- Source-order entry and field occurrences.
- Byte spans for blocks, entries, and fields.
- Field-value validation against the original delimiter mode.
- Writeback that keeps unrelated raw blocks in place.

## Address duplicates by occurrence

Direct mapping lookup requires one matching occurrence. Use `get_all` when a key or field name appears more than once:

```python
document = rk.BibDocument.parse(
    """
@article{same, title={First}}
@article{same, title={Second}}
"""
)

second = document.entries.get_all("same")[1]
second.fields["title"].value = "Updated second title"
```

`occurrence_keys()` includes one key per source occurrence. `unique_keys()` includes each distinct key name once, even when that name has duplicate occurrences. `get_unique(key)` returns `None` for a missing name and raises `RefkitError` for an ambiguous name. `occurrences()` returns the source-order objects themselves.

## Move between the models deliberately

Parse the same source separately when a workflow needs both contracts:

```python
library = rk.Library.parse_bibtex(source, recovery="report")
raw = rk.BibDocument.parse(source)
```

The two objects own separate state. An edit in `raw` changes its writeback result. Parse `raw.to_bibtex()` into a new `Library` when normalized rendering must reflect that edit.

Continue with [Parsing and Recovery](/concepts/parsing-and-recovery) or [Edit Raw BibTeX](/guides/edit-bibtex).
