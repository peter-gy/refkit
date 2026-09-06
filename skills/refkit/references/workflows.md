# RefKit Workflows

## Parse and inspect normalized data

```python
import refkit as rk

source = """
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  year = {2024}
}
"""

library = rk.Library.parse_bibtex(source, recovery="report")
rows = library.project(["key", "entry_type", "title", "date", "doi"])
diagnostics = list(library.diagnostics)
```

`Library` preserves normalized entry order. Recovery diagnostics describe repairs applied before typed entries were accepted.

## Render an ordered citation document

```python
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
rendered = document.render(
    [
        rk.Citation(id="introduction", citation="doe2024"),
        rk.Citation(
            id="detail",
            citation=rk.CitationGroup([rk.Cite("doe2024", locator="12", label="page")]),
        ),
    ]
)

citation_text = rendered["detail"].text
bibliography_html = rendered.bibliography.html
```

Each `render` call creates fresh citation processor state. Keep related citations in one ordered call.

## Edit raw BibTeX while preserving surrounding source

```python
duplicate_source = """% Keep this comment.
@article{doe2024, TITLE={First title}, year={2024}}
@article{doe2024, tItLe={Second title}, year={2025}}
"""
raw = rk.BibDocument.parse(duplicate_source)
entries = raw.entries.get_all("doe2024")
if len(entries) != 2:
    raise ValueError(f"expected two doe2024 entries, found {len(entries)}")

entry = entries[1]
field = entry.fields.get_all("title")[0]
field.value = "Corrected title"

preview = raw.to_bibtex()
print(preview)
```

Entry keys are case-sensitive. Field lookup is case-insensitive and preserves the source spelling of the field name. `get_all` makes duplicate occurrence selection explicit. `to_bibtex()` returns preview text and performs no filesystem write.

After inspecting `preview`, commit the preserving write to the intended path:

```python
raw.write("references.bib")
```

## Format and inspect warnings

```python
duplicate_source = """
@article{same, title={First}}
@book{same, title={Second}}
"""
result = rk.tidy_bibtex(
    duplicate_source,
    options=rk.TidyOptions(
        duplicates=["key"],
        sort_fields=True,
        wrap=88,
    ),
)

formatted = result.bibtex
warnings = [
    {"code": warning.code, "rule": warning.rule, "message": warning.message}
    for warning in result.warnings
]
```

`duplicates=None` skips duplicate detection. `duplicates=["key"]` emits a `duplicate_entry` warning for repeated citation keys. `TidyResult.count` records parsed entry occurrences before duplicate merges.

## Process bibliography columns with Polars

Install `polars-refkit` separately, then import it once to register the expression namespace:

```python
import polars as pl
import polars_refkit

frame = pl.DataFrame({"bibtex": [source], "key": ["doe2024"]})
result = frame.select(
    citation_from_column=pl.col("bibtex").refkit.cite("key"),
    citation_from_literal=pl.col("bibtex").refkit.cite(pl.lit("doe2024")),
    entries=pl.col("bibtex").refkit.entries(fields=["key", "entry_type", "title"]),
)
```

A plain string argument names a column. Wrap literal source or citation keys with `pl.lit(...)`. Importing `polars_refkit` registers `pl.Expr.refkit`. Citation expressions return null for row-local parse failures, missing keys, or rendering failures.
