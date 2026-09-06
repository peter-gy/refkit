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
        rk.Citation("introduction", "doe2024"),
        rk.Citation(
            "detail",
            rk.CitationGroup([rk.Cite("doe2024", locator="12", label="page")]),
        ),
    ]
)

citation_text = rendered["detail"].text
bibliography_html = rendered.bibliography.html
```

Each `render` call creates fresh citation processor state. Keep related citations in one ordered call.

## Edit raw BibTeX while preserving surrounding source

```python
raw = rk.BibDocument.parse(source)
entry = raw.entries.get_all("doe2024")[0]
field = entry.fields.get_all("title")[0]
field.value = "Corrected title"

preview = raw.to_bibtex()
preview
```

Entry keys are case-sensitive. Field lookup is case-insensitive. `get_all` makes duplicate occurrence selection explicit.

After inspecting `preview`, commit the preserving write to the intended path:

```python
raw.write("references.bib")
```

## Format and inspect warnings

```python
result = rk.tidy_bibtex(
    source,
    options=rk.TidyOptions(sort_fields=True, wrap=88),
)

formatted = result.bibtex
warnings = [
    {"code": warning.code, "rule": warning.rule, "message": warning.message}
    for warning in result.warnings
]
```

`TidyResult.count` records parsed entry occurrences before duplicate merges.

## Process bibliography columns with Polars

Install `polars-refkit` separately, then import it once to register the expression namespace:

```python
import polars as pl
import polars_refkit

frame = pl.DataFrame({"bibtex": [source], "key": ["doe2024"]})
result = frame.select(
    citation=pl.col("bibtex").refkit.cite("key"),
    entries=pl.col("bibtex").refkit.entries(
        fields=["key", "entry_type", "title"]
    ),
)
```

A plain string argument names a column. Wrap literal source or citation keys with `pl.lit`.
