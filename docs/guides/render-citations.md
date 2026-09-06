---
description: Render named citations and cited or full bibliographies with bundled or explicit CSL styles.
---

# Render Citations

`Document` renders a complete ordered citation document against one normalized library, Citation Style Language style, and locale.

## Render named citations

```python
import refkit as rk

library = rk.Library.read("references.bib")
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")

result = document.render(
    [
        rk.Citation("opening", "doe2024"),
        rk.Citation(
            "details",
            rk.CitationGroup(
                [
                    rk.Cite("doe2024", locator="12", label="page"),
                    "roe2022",
                ]
            ),
        ),
    ]
)

print(result["opening"].text)
print(result["details"].html)
print(result.bibliography.text)
```

Citation IDs must be unique inside one call. The full order can affect numbering, subsequent-citation rules, disambiguation, and bibliography contents.

## Add a locator

`Cite` accepts a citation key plus optional `locator` and `label` values:

```python
page = rk.Cite("doe2024", locator="12", label="page")
```

The style controls the visible form. An unknown locator label raises `ValueError` during rendering.

## Use a Path Helper

The path helper loads the library and style for one result:

```python
citation = rk.cite("references.bib", "doe2024", style="ieee")
print(citation.text)
```

Pass a prepared `Style`, `Cite`, or `CitationGroup` when the helper needs richer input. The helper reads its first argument as a path. Use `Library.parse_bibtex` plus `Document` for in-memory source.

## Render a bibliography

Use the boundary that matches the document:

```python
cited = document.cited_bibliography(
    [rk.Citation("opening", "doe2024")]
)
complete = document.full_bibliography()
```

`cited` contains entries referenced by the ordered citation document. `complete` contains every entry in the library.

The path helper renders the complete library:

```python
complete = rk.full_bibliography(
    "references.bib",
    style="chicago-author-date",
)
```

## Load an explicit style

```python
style = rk.Style.from_path("journal.csl")
style_from_memory = rk.Style.from_xml(csl_xml)
```

Independent CSL styles are accepted. A dependent style that requires parent resolution raises `ValueError`. Use `Style.load(name)` when the style is part of the bundled archive.

## Render safely for the web

`Rendered.html` emits CSL markup and escapes bibliography data. Link nodes allow safe URL schemes. A URL with an unsafe scheme remains visible as text without becoming a link.

Use `Rendered.tree` when an application needs to control its own element creation and styling. Read [Data Shapes](/reference/data-shapes) for the exact node records.
