---
description: Build an ordered citation document and inspect its citations, bibliography, HTML, text, or render tree.
---

# Citation Rendering

Citation rendering transforms normalized entries through a Citation Style Language (CSL) style. One render operation owns the full ordered citation list and the bibliography derived from it.

## Build a citation document

```python
import refkit as rk

library = rk.Library.parse_bibtex(
    """
@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}
@book{roe2022, author={Roe, Richard}, title={Batch References}, year={2022}}
"""
)
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")

rendered = document.render(
    [
        rk.Citation("intro", "doe2024"),
        rk.Citation(
            "detail",
            rk.CitationGroup([rk.Cite("doe2024", locator="12", label="page"), "roe2022"]),
        ),
    ]
)
```

The rendering nouns are:

- `Cite`: one citation item with a key and optional locator.
- `CitationGroup`: one or more items rendered as one citation cluster.
- `Citation`: a group plus a unique result ID.
- `Document`: a library, style, and locale prepared for rendering.
- `RenderedDocument`: named citation outputs plus the cited bibliography.

## Keep the full order together

`Document.render` creates fresh render state for each call. Pass the complete ordered citation list when later citations depend on earlier disambiguation or numbering.

The returned `citation_order` preserves the input IDs. Use `rendered[id]` or `rendered.citations[id]` to read a named output.

## Choose a bibliography boundary

| Operation | Bibliography contents |
| --- | --- |
| `Document.render(citations).bibliography` | Entries cited by that render call. |
| `Document.cited_bibliography(citations)` | The same cited bibliography without returning citation outputs. |
| `Document.full_bibliography()` | Every entry in the library. |

## Choose an output representation

Every `Rendered` value exposes:

- `text` or `to_text()` for plain text.
- `html` or `to_html()` for escaped HTML.
- `tree` or `to_tree()` for structured nodes and bibliography records.

The tree keeps formatting and link structure available to a host renderer. Read [Data Shapes](/reference/data-shapes) for the node contract.

## Load styles and locales

`Style.load(name)` resolves a bundled style such as `apa`, `ieee`, or `chicago-author-date`. `Style.from_xml(xml)` and `Style.from_path(path)` prepare explicit CSL XML.

`Locale.load(code)` validates a bundled locale. `Document(..., locale="en-US")` accepts a locale code directly. Use a `Locale` object when validation should happen before document construction.

Continue with [Render Citations](/guides/render-citations) for file-based helpers and complete examples.
