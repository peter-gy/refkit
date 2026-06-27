# Migration Guide

Refkit replaces the common split between a CSL renderer and a BibTeX repair library with one Python API backed by Rust.

## Package Split

Install `refkit` for Python citation workflows:

```bash
pip install refkit
```

`refkit` is pure Python and depends on `refkit-core==0.0.1`.
`refkit-core` contains the native Rust/PyO3 extension as `refkit_core._refkit_core`.
Importing `refkit` checks that the installed `refkit-core` version matches the version required by `refkit`.

```python
import refkit as rk

assert rk.check_refkit_core_version()
```

Runtime metadata such as `rk.build_info` and `rk.build_mode` is available from the `refkit` package.

## From citeproc-py

Use `Library`, `Style`, and `Document` for the main render flow.

```python
import refkit as rk

library = rk.Library.parse_bibtex(
    """
@article{doe2024, title={Fast Citations}, year={2024}}
@book{roe2022, title={Batch References}, year={2022}}
"""
)
style = rk.Style.load("apa")
doc = rk.Document(library, style, locale="en-US")
rendered = doc.render([rk.Citation("intro", "doe2024")])

print(rendered["intro"].text)
print(rendered.bibliography.html)
```

The closest citeproc-py roles map to these refkit objects:

| citeproc-py role | refkit object |
| --- | --- |
| BibTeX source adapter | `Library.read("refs.bib")` or `Library.parse_bibtex(source)` |
| JSON source adapter | Use a BibTeX, BibLaTeX, or Hayagriva YAML source. CSL JSON input is outside the current API. |
| `CitationStylesStyle` | `Style` |
| `CitationItem` | `Cite` |
| Citation cluster | `CitationGroup` inside a named `Citation` |
| `CitationStylesBibliography` | `Document` |
| Formatter output | `Rendered.text`, `Rendered.html`, `Rendered.tree` |

`Document.render` accepts named `Citation` objects. Use `CitationGroup` inside a `Citation` when one rendered citation contains multiple citation items.

```python
rendered = doc.render(
    [
        rk.Citation(
            "detail",
            rk.CitationGroup([rk.Cite("doe2024", locator="12", label="page"), "roe2022"]),
        )
    ]
)
```

Missing references raise `MissingReferenceError`. There is no callback-based missing-reference hook in the main API.

```python
try:
    doc.render([rk.Citation("missing", "missing-key")])
except rk.MissingReferenceError:
    ...
```

Refkit renders text, HTML, and a structured tree. citeproc-py workflows that depend on reStructuredText output or custom formatter modules need a project-specific adapter around `Rendered.tree` or `Rendered.text`.

## From python-bibtexparser

Use `BibDocument` when raw `.bib` structure must survive an edit.

```python
import refkit as rk

raw = rk.BibDocument.read("refs.bib")
raw.entries["doe2024"].fields["title"].value = "Corrected title"
raw.write("refs.bib")
```

The main raw-document concepts map as follows:

| python-bibtexparser concept | refkit object |
| --- | --- |
| Parsed library blocks | `BibDocument.blocks` |
| Entry block | `BibEntry` |
| Entry fields | `BibEntry.fields` |
| Comments | `BibDocument.comments` and `BibDocument.blocks` |
| Preamble | `BibDocument.preamble` and `BibDocument.blocks` |
| String definitions | `BibDocument.strings` and `BibDocument.blocks` |
| Failed parse blocks | `BibDocument.failed_blocks` |
| File write | `BibDocument.write(path)` |

Duplicate entry keys and duplicate field names are source-order lists in refkit. Direct lookup raises when the key is ambiguous.

```python
raw = rk.BibDocument.read("refs.bib")

second = raw.entries.get_all("doe2024")[1]
second.fields.get_all("title")[0].value = "Corrected title"
raw.write("refs.bib")
```

`BibDocument` edits existing field values. Workflows that add, remove, reorder, or run BibTeX middleware transforms should stay on python-bibtexparser until refkit exposes those contracts.

## In-Memory Sources

Use `Library.parse_bibtex` or `Library.parse_yaml` for normalized in-memory rendering. Use `BibDocument.parse` for raw in-memory repair.

```python
library = rk.Library.parse_bibtex("@article{doe2024, title={Fast Citations}, year={2024}}")
raw = rk.BibDocument.parse("@article{doe2024, title={Old}}\n")
```

One-off helpers read from paths. They are intended for scripts where the bibliography already lives on disk.
