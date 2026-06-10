# refkit

Refkit parses bibliography files, renders CSL citations, and edits raw BibTeX documents from one Python API.

It supports CPython 3.11 through 3.14. Wheels use the Python 3.11 stable ABI.

```python
import refkit as rk

library = rk.Library.read("refs.bib")
style = rk.Style.load("apa")
doc = rk.Document(library, style, locale="en-US")

first = doc.cite("doe2024")
second = doc.cite([rk.Cite("doe2024", locator="12", label="page"), "roe2022"])

print(first.text)
print(doc.bibliography().html)
```

Use `doc.bibliography(all=True)` to render every entry in the library, including entries that were not cited.

One-off calls parse the library, load the style, render, and return a `Rendered` object.

```python
rk.cite("refs.bib", "doe2024", style="ieee").text
rk.bibliography("refs.bib", style="chicago-author-date").html
```

`Library` is the normalized citation database. Use it for rendering, selection, and bulk export.

```python
library = rk.Library.read("refs.yaml")

for entry in library.select("article > periodical[volume]"):
    print(entry.key, entry.title, entry.parent.title)
```

Use `Library.parse` when the bibliography source is already in memory.

```python
library = rk.Library.parse("@article{doe2024, title={Fast Citations}}")
assert library.get("doe2024").title == "Fast Citations"
```

`BibDocument` is the raw `.bib` document model. Use it when comments, preambles, strings, failed blocks, order, or source spans need to survive an edit.

```python
raw = rk.BibDocument.read("refs.bib")
raw.entries["doe2024"].fields["title"].value = "Corrected title"
raw.write("refs.bib")
```

Use `BibDocument.parse` for in-memory repair flows. Call `write(path)` because parsed documents do not have a source path.

```python
raw = rk.BibDocument.parse("% note\n@article{doe2024, title={Old}}\n")
raw.entries["doe2024"].fields["title"].value = "Corrected title"
raw.write("refs.bib")
```

## Development

```bash
uv sync
uv run maturin develop
uv run pytest
```
