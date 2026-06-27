# refkit

`refkit` reads BibTeX, BibLaTeX, and Hayagriva YAML, renders CSL citations, and edits raw BibTeX documents from Python.

## Install

```bash
pip install refkit
```

`refkit` is pure Python and depends on the exact matching `refkit-core` release.
`refkit-core` contains the Rust/PyO3 extension as `refkit_core._refkit_core`, including PyEmscripten wheels for the Python 3.14 Pyodide runtime.

`refkit` supports CPython 3.11 through 3.14. Native wheels from `refkit-core` use the Python 3.11 stable ABI.

## Render A Citation

```python
import refkit as rk

library = rk.Library.parse_bibtex(
    """
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  journal = {Journal of Citation Tests},
  year = {2024}
}
@book{roe2022,
  author = {Roe, Richard},
  title = {Batch References},
  publisher = {Example Press},
  year = {2022}
}
"""
)
style = rk.Style.load("apa")
doc = rk.Document(library, style, locale="en-US")

rendered = doc.render(
    [
        rk.Citation("intro", "doe2024"),
        rk.Citation(
            "detail",
            rk.CitationGroup([rk.Cite("doe2024", locator="12", label="page"), "roe2022"]),
        ),
    ]
)

print(rendered["intro"].text)
print(rendered["detail"].text)
print(rendered.bibliography.text)
```

Expected output:

```text
(Doe, 2024)
(Doe, 2024, p. 12; Roe, 2022)
Doe, J. (2024). Fast Citations. Journal of Citation Tests.
Roe, R. (2022). Batch References. Example Press.
```

`Document.render` renders the whole citation document at once. It returns `RenderedDocument`, where `rendered["intro"]` and `rendered["detail"]` are named citation outputs and `rendered.bibliography` is the cited bibliography for those citations.
`Cite` names one citation item. `CitationGroup` renders several items as one citation. `Citation(id, group)` gives that rendered citation a stable lookup name. Citation ids must be unique inside one `Document.render` call.

For one-off scripts, pass a bibliography path directly:

```python
rk.cite("refs.bib", "doe2024", style="ieee").text
rk.full_bibliography("refs.bib", style="chicago-author-date").html
```

Use `Library.parse_bibtex`, `Library.parse_yaml`, and `Document` when the bibliography source is already in memory or when several citations share the same library and style.

## Capabilities

| Capability | Python surface |
| --- | --- |
| Read normalized bibliography data | `Library.read`, `Library.parse_bibtex`, `Library.parse_yaml` |
| Render citations | `Document.render`, `Citation`, `Cite`, `CitationGroup`, `cite` |
| Render bibliographies | `Document.cited_bibliography`, `Document.full_bibliography`, `full_bibliography` |
| Load styles and locales | `Style.load`, `Style.from_path`, `Style.from_xml`, `Locale.load` |
| Inspect entries | mapping access, `keys`, `get`, `get_many`, `select`, `project`, `to_dicts` |
| Edit raw BibTeX | `BibDocument.read`, `BibDocument.parse`, field assignment, `write` |
| Inspect rendered output | `Rendered.text`, `Rendered.html`, `Rendered.tree` |

## Input Formats

| API | Input | Result |
| --- | --- | --- |
| `Library.read(path)` | `.bib` | Normalized citation library from BibTeX or BibLaTeX. |
| `Library.read(path)` | `.yaml`, `.yml` | Normalized citation library from Hayagriva YAML. |
| `Library.parse_bibtex(source)` | BibTeX or BibLaTeX string | Normalized citation library. |
| `Library.parse_yaml(source)` | Hayagriva YAML string | Normalized citation library. |
| `BibDocument.read(path)` | `.bib` | Raw document model with comments, preambles, strings, failed blocks, order, spans, and editable fields. |
| `Style.load(name)` | Bundled style name such as `apa` | CSL style for rendering. |
| `Style.from_path(path)` | Independent CSL XML file | CSL style for rendering. |
| `Style.from_xml(xml)` | Independent CSL XML string | CSL style for rendering. |
| `Locale.load(code)` | Bundled locale code such as `en-US` | Locale object for rendering. |

Hayagriva YAML is a mapping from citation keys to entry mappings:

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
library = rk.Library.parse_yaml(
    """
doe2024:
  type: Article
  title: Refkit for Bibliographies
  date: 2024
"""
)
```

## Inspect A Library

`Library` is the normalized citation database. Use it for rendering, selectors, mapping access, and bulk export.

```python
library = rk.Library.read("refs.bib")

print(library.keys())
print(library.project(["key", "title", "doi", "volume"]))
print(library.to_dicts())
```

`Library.select` accepts Hayagriva selector strings:

```python
for entry in library.select("article > periodical[volume]"):
    print(entry.key, entry.title, entry.parents[0].title)
```

## Edit Raw BibTeX

`BibDocument` preserves the raw `.bib` structure that normalized rendering does not need: comments, preambles, string definitions, failed blocks, order, and source spans.

```python
raw = rk.BibDocument.read("refs.bib")
raw.entries["doe2024"].fields["title"].value = "Corrected title"
raw.write("refs.bib")
```

Direct map lookup requires one matching entry key and one matching field name. `unique_keys()` returns one key per name. `occurrence_keys()` returns keys in source order, including duplicates. When a file contains duplicates, choose the source-order occurrence explicitly:

```python
raw = rk.BibDocument.read("refs.bib")

second_entry = raw.entries.get_all("doe2024")[1]
second_entry.fields.get_all("title")[0].value = "Corrected title"
raw.write("refs.bib")
```

## Inspect Rendered Output

`Document.render`, `Document.cited_bibliography`, `Document.full_bibliography`, `cite`, and `full_bibliography` return rendered values.

```python
rendered = doc.render([rk.Citation("intro", "doe2024")])
citation = rendered["intro"]

print(citation.text)
print(citation.html)
print(citation.tree)
```

`Rendered.tree` returns structured nodes for text, links, element metadata, transparent citation fragments, and bibliography entries.

## Use With Polars

Install `polars-refkit` when BibTeX source lives in a dataframe and the result should stay in a Polars query plan.

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
    citation=pl.col("bibtex").refkit.cite(pl.col("key")),
    each_citation=pl.col("bibtex").refkit.cite_each(pl.col("keys")),
    grouped_citation=pl.col("bibtex").refkit.cite_group(pl.col("keys")),
    count=pl.col("bibtex").refkit.entry_count(),
    keys=pl.col("bibtex").refkit.keys(),
    entries=pl.col("bibtex").refkit.entries(),
)
```

## Development

```bash
uv sync --all-packages --group dev
uv run maturin develop --manifest-path packages/refkit-core/Cargo.toml
uv run pytest packages/refkit/tests --no-cov
```

The workspace also provides:

```bash
make lint
make typecheck
make test
make rust
make build
```

## License

`refkit` is licensed under the Apache License, Version 2.0, available in [LICENSE](LICENSE).
