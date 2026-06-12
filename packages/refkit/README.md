# refkit

`refkit` reads BibTeX, BibLaTeX, and Hayagriva YAML into a citation library, renders CSL citations, and edits raw BibTeX documents from Python.

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

Use `doc.bibliography(all=True)` to render every entry in the library. Without `all=True`, the bibliography contains entries that have been cited through the document.

`refkit` supports CPython 3.11 through 3.14. Wheels use the Python 3.11 stable ABI.

## Capabilities

| Capability | Python surface |
| --- | --- |
| Normalized bibliography input | `Library.read` and `Library.parse` |
| Raw BibTeX document editing | `BibDocument.read`, `BibDocument.parse`, field assignment, and `write` |
| Citation style input | `Style.load`, `Style.from_path`, `Style.from_xml`, and `Locale.load` |
| Citation rendering | `Document.cite` and `cite` |
| Bibliography rendering | `Document.bibliography` and `bibliography` |
| Rendered output access | `Rendered.text`, `Rendered.html`, and `Rendered.tree` |
| Entry inspection | `keys`, `get`, `get_many`, `select`, `project`, and `to_dicts` |
| Error contracts | `MissingReferenceError`, `RefkitError`, `ValueError`, `KeyError`, and `TypeError` at the public call site |

## One-Off Rendering

One-off helpers read a bibliography path, load the style, render, and return a `Rendered` object.

```python
rk.cite("refs.bib", "doe2024", style="ieee").text
rk.bibliography("refs.bib", style="chicago-author-date").html
```

Use `Library.parse` and `Document` when the bibliography source is already in memory or when several citations share the same library and style.

## Supported Formats

| API | Input | Result |
| --- | --- | --- |
| `Library.read(path)` | `.bib` | Normalized citation library from BibTeX or BibLaTeX. |
| `Library.read(path)` | `.yaml`, `.yml` | Normalized citation library from Hayagriva YAML. |
| `Library.parse(source, format="bibtex")` | BibTeX or BibLaTeX string | Normalized citation library. |
| `Library.parse(source, format="yaml")` | Hayagriva YAML string | Normalized citation library. |
| `BibDocument.read(path)` | `.bib` | Raw document model for comments, preambles, strings, failed blocks, order, spans, and field edits. |
| `Style.load(name)` | Bundled style name such as `apa` | CSL style for rendering. |
| `Style.from_path(path)` | Independent CSL XML file | CSL style for rendering. |
| `Style.from_xml(xml)` | Independent CSL XML string | CSL style for rendering. |
| `Locale.load(code)` | Bundled locale code such as `en-US` | Locale object for rendering. |

## Library

`Library` is the normalized citation database. Use it for rendering, selectors, mapping access, and bulk export.

```python
library = rk.Library.read("refs.bib")

print(library.keys())
print(library.project(["key", "title", "doi", "volume"]))
print(library.to_dicts())
```

`Library.select` accepts Hayagriva selector strings.

```python
for entry in library.select("article > periodical[volume]"):
    print(entry.key, entry.title, entry.parent.title)
```

Hayagriva YAML is a mapping from citation keys to entry mappings.

```yaml
doe2024:
  type: Article
  author: Doe, Jane
  title: Refkit for Bibliographies
  date: 2024
  parent:
    type: Periodical
    title: Journal of Citation Systems
    volume: 12
```

```python
source = """
doe2024:
  type: Article
  title: Refkit for Bibliographies
  date: 2024
"""

library = rk.Library.parse(source, format="yaml")
```

## BibDocument

`BibDocument` is the raw BibTeX document model. Use it when comments, preambles, strings, failed blocks, order, or source spans need to survive an edit.

```python
raw = rk.BibDocument.read("refs.bib")
raw.entries["doe2024"].fields["title"].value = "Corrected title"
raw.write("refs.bib")
```

Direct map lookup requires a unique entry key and a unique field name. When a file contains duplicates, choose the source-order occurrence explicitly.

```python
raw = rk.BibDocument.read("refs.bib")

second_entry = raw.entries.get_all("doe2024")[1]
second_entry.fields.get_all("title")[0].value = "Corrected title"
raw.write("refs.bib")
```

## Rendered

`Document.cite`, `Document.bibliography`, `cite`, and `bibliography` return `Rendered`.

```python
rendered = doc.cite("doe2024")

print(rendered.text)
print(rendered.html)
print(rendered.tree)
```

## Polars

Install `polars-refkit` when BibTeX source lives in a Polars dataframe.

```python
import polars as pl
import polars_refkit as prk

df = pl.DataFrame(
    {
        "bibtex": ["@article{doe2024, title={Fast Citations}, year={2024}}"],
        "key": ["doe2024"],
        "keys": [["doe2024"]],
    }
)

out = df.select(
    citation=prk.cite("bibtex", "key"),
    citations=prk.cite_sequence("bibtex", "keys"),
    count=prk.entry_count("bibtex"),
    keys=prk.keys("bibtex"),
    entries=prk.entries("bibtex"),
)
```

## Development

```bash
uv sync --all-packages --group dev
uv run maturin develop --manifest-path packages/refkit/Cargo.toml
uv run pytest packages/refkit/tests --no-cov
```

The workspace also provides:

```bash
make lint
make typecheck
make test
make rust
make package-check
```

## License

`refkit` is licensed under the Apache License, Version 2.0, available in [LICENSE](LICENSE).
