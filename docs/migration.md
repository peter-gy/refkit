---
description: Move from refkit_core, citeproc-py, or python-bibtexparser to current RefKit objects and workflows.
---

# Migration Guide

Use the section that matches the starting API. Each path maps the old ownership and output model to current RefKit objects.

## Replace `refkit_core` imports

RefKit 0.0.4 packages the native extension inside the `refkit` distribution. Import every public object from `refkit`:

```python
import refkit as rk

library = rk.Library.parse_bibtex("@article{doe2024, title={Fast Citations}}")
```

Replace direct `refkit_core` imports with `refkit`. Runtime metadata remains available as `rk.build_info` and `rk.build_mode`.

Replace `Library.to_dicts()` with a projection:

```python
rows = library.project(["key", "entry_type", "title", "date", "doi", "volume"])
```

In Polars, replace `to_hayagriva_json()` with `entries(fields=...)`.

## Move from citeproc-py

Use `Library`, `Style`, and `Document` for a complete ordered render operation:

```python
import refkit as rk

library = rk.Library.parse_bibtex(
    """
@article{doe2024, title={Fast Citations}, year={2024}}
@book{roe2022, title={Batch References}, year={2022}}
"""
)
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
rendered = document.render([rk.Citation("intro", "doe2024")])

print(rendered["intro"].text)
print(rendered.bibliography.html)
```

| citeproc-py role | RefKit contract |
| --- | --- |
| BibTeX source adapter | `Library.read` or `Library.parse_bibtex` |
| `CitationStylesStyle` | `Style` |
| `CitationItem` | `Cite` |
| Citation cluster | `CitationGroup` inside a named `Citation` |
| `CitationStylesBibliography` | `Document` |
| Formatter output | `Rendered.text`, `Rendered.html`, or `Rendered.tree` |

Pass the complete ordered citation document to one call:

```python
rendered = document.render(
    [
        rk.Citation(
            "detail",
            rk.CitationGroup([rk.Cite("doe2024", locator="12", label="page"), "roe2022"]),
        )
    ]
)
```

Missing references raise `MissingReferenceError`. RefKit returns text, HTML, and a structured tree. A workflow that produces another format can render from the tree or text in application code.

RefKit accepts BibTeX, BibLaTeX, and Hayagriva YAML as normalized inputs. Convert Citation Style Language JSON into one of those inputs before constructing a `Library`.

## Move from python-bibtexparser

Use `BibDocument` when raw `.bib` structure must survive an existing-field edit:

```python
import refkit as rk

document = rk.BibDocument.read("references.bib")
document.entries["doe2024"].fields["title"].value = "Corrected title"
document.write("references.bib")
```

| python-bibtexparser concept | RefKit contract |
| --- | --- |
| Parsed library blocks | `BibDocument.blocks` |
| Entry block | `BibEntry` |
| Entry fields | `BibEntry.fields` |
| Comments | `BibDocument.comments` and `blocks` |
| Preamble | `BibDocument.preamble` and `blocks` |
| String definitions | `BibDocument.strings` and `blocks` |
| Failed parse blocks | `BibDocument.failed_blocks` |
| File write | `BibDocument.write(path)` |

Address duplicate entries and fields by occurrence:

```python
second = document.entries.get_all("doe2024")[1]
second.fields.get_all("title")[0].value = "Corrected title"
```

`BibField.value` edits existing field values. Keep python-bibtexparser in workflows that create, delete, or reorder raw blocks until the RefKit API owns those mutations.

## Choose in-memory entry points

Use `Library.parse_bibtex` or `Library.parse_yaml` for normalized source already in memory. Use `BibDocument.parse` for raw BibTeX already in memory.

One-call `cite` and `full_bibliography` helpers read a filesystem path.
