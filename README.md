# refkit

Refkit reads BibTeX, BibLaTeX, and Hayagriva YAML into a normalized citation library, renders CSL citations, and edits raw BibTeX documents from one Python API.

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

One-off calls read a bibliography path, load the style, render, and return a `Rendered` object. Use `Library.parse` and `Document` for in-memory sources.

```python
rk.cite("refs.bib", "doe2024", style="ieee").text
rk.bibliography("refs.bib", style="chicago-author-date").html
```

`rk.cite` accepts the same citation group shapes as `Document.cite`: a key string, a `Cite`, or an iterable of keys and `Cite` objects.

## Supported Formats

| API | Input | Selection | Result |
| --- | --- | --- | --- |
| `Library.read(path)` | `.bib` | File extension | Normalized citation library from BibTeX or BibLaTeX. |
| `Library.read(path)` | `.yaml`, `.yml` | File extension | Normalized citation library from Hayagriva YAML. |
| `Library.parse(source, format=...)` | `bib`, `bibtex`, `biblatex` | Explicit `format` argument | Normalized citation library from an in-memory BibTeX or BibLaTeX string. |
| `Library.parse(source, format=...)` | `yaml`, `yml` | Explicit `format` argument | Normalized citation library from an in-memory Hayagriva YAML string. |
| `BibDocument.read(path)` | `.bib` source | Raw BibTeX parser | Raw document model for comments, preambles, string definitions, failed blocks, order, spans, and field edits. |
| `BibDocument.parse(source)` | BibTeX source string | Raw BibTeX parser | Raw document model for in-memory repair flows. |
| `Style.load(name)` | Bundled style name such as `apa` | Hayagriva style archive | CSL style for rendering. |
| `Style.from_xml(xml)` | Independent CSL XML string | XML parser | CSL style for rendering. |
| `Style.from_path(path)` | Independent CSL XML file | File path | CSL style for rendering. |
| `Locale.load(code)` | Bundled locale code such as `en-US` | Hayagriva locale archive | Locale object accepted by `Document`, `cite`, and `bibliography`. |

`Library` is the normalized citation database. Use it for rendering, selection, mapping access, and bulk export.

```python
library = rk.Library.read("refs.bib")

print(library.keys())
print(library.project(["key", "title", "doi", "volume"]))
print(library.to_dicts())
```

## Hayagriva YAML

Hayagriva YAML is a YAML document whose top level is a mapping from citation keys to entry mappings. Each entry has a `type`, citation fields such as `title`, `author`, `date`, and optional `parent` data for the containing journal, book, conference, video, or other source.

```yaml
doe2024:
  type: Article
  author: Doe, Jane
  title: Refkit for Bibliographies
  date: 2024
  page-range: 1-20
  serial-number:
    doi: 10.1234/refkit.2024
  parent:
    type: Periodical
    title: Journal of Citation Systems
    volume: 12
```

`Library.select` accepts Hayagriva selector strings. The selector below returns articles whose parent is a periodical with a volume.

```python
library = rk.Library.read("refs.yaml")

for entry in library.select("article > periodical[volume]"):
    print(entry.key, entry.title, entry.parent.title)
```

Use `Library.parse` when the bibliography source is already in memory. Pass `format="yaml"` or `format="yml"` for Hayagriva YAML.

```python
source = """
doe2024:
  type: Article
  title: Refkit for Bibliographies
  date: 2024
"""

library = rk.Library.parse(source, format="yaml")
assert library.get("doe2024").title == "Refkit for Bibliographies"
```

## BibTeX And BibLaTeX

`Library.read("refs.bib")` and `Library.parse(..., format="biblatex")` normalize BibTeX or BibLaTeX records for rendering and selection.
Malformed blocks are skipped during normalization by default. Pass `diagnostics=True` to inspect recovery decisions, or `strict=True` when a parse failure should stop the call.

```python
library = rk.Library.parse(
    "@article{doe2024, title={Fast Citations}, year={2024}}",
    format="bibtex",
)
assert library.get("doe2024").title == "Fast Citations"
```

`BibDocument` is the raw BibTeX document model. Use it when comments, preambles, strings, failed blocks, order, or source spans need to survive an edit.

```python
raw = rk.BibDocument.read("refs.bib")
raw.entries["doe2024"].fields["title"].value = "Corrected title"
raw.write("refs.bib")
```

Direct map lookup requires a unique entry key and a unique field name inside that entry. When a `.bib` file contains duplicate entry keys or duplicate fields, choose the source-order occurrence explicitly.

```python
raw = rk.BibDocument.read("refs.bib")

second_entry = raw.entries.get_all("doe2024")[1]
second_entry.fields.get_all("title")[0].value = "Corrected title"
```

Use `BibDocument.parse` for in-memory repair flows. Call `write(path)` because parsed documents do not have a source path.

```python
raw = rk.BibDocument.parse("% note\n@article{doe2024, title={Old}}\n")
raw.entries["doe2024"].fields["title"].value = "Corrected title"
raw.write("refs.bib")
```

## API Contracts And Migration

The repository docs include focused guides for the public shapes that matter during integration:

| Guide | Scope |
| --- | --- |
| [API contracts](https://github.com/petergy/refkit/blob/main/docs/api-contracts.md) | One-off helper inputs, `Rendered.tree`, raw block dictionaries, and public error behavior. |
| [Migration guide](https://github.com/petergy/refkit/blob/main/docs/migration.md) | Replacing common citeproc-py render flows and python-bibtexparser raw repair flows. |

## Styles And Locales

`Style.load(name)` loads a bundled Hayagriva archive style by name.

```python
style = rk.Style.load("apa")
```

Use `Style.from_xml` or `Style.from_path` when you already have independent CSL XML.

```python
style = rk.Style.from_path("style.csl")
```

`Locale.load(code)` loads a bundled Hayagriva locale. Passing a locale object or locale code to `Document` controls localized terms and date rendering.

```python
locale = rk.Locale.load("en-US")
doc = rk.Document(library, style, locale=locale)
```

## Development

```bash
uv sync
uv run maturin develop
uv run pytest
```

Local quality gates:

```bash
make lint
make typecheck
make test
make rust
make rust-floor
make package-check
```

`make all` runs the local lint, type-check, test, Rust, Rust floor, and package-content gates.

Release checks:

```bash
make release-smoke
make dependency-provenance
make rust-floor
make advisory
```

`make release-smoke` builds the wheel, inspects the sdist and wheel contents, and imports the built wheel in fresh Python 3.11, 3.12, 3.13, and 3.14 environments. `make dependency-provenance` checks the locked Typst crate paths and the YAML parser dependency path. `make rust-floor` checks the declared Rust 1.85 floor. `make advisory` is opt-in because it may download advisory databases and audit tools. See the [release validation guide](https://github.com/petergy/refkit/blob/main/docs/release.md) in the repository for the full contract.

## Acknowledgements

Refkit builds on three Typst Rust crates:

| Crate | Repository | Role |
| --- | --- | --- |
| `hayagriva` | <https://github.com/typst/hayagriva> | Normalized bibliography model, Hayagriva YAML parsing, CSL rendering, bundled styles, and bundled locales. |
| `biblatex` | <https://github.com/typst/biblatex> | BibTeX and BibLaTeX parsing, field typing, cross-reference handling, and writer support. |
| `citationberg` | <https://github.com/typst/citationberg> | CSL style and locale parsing used through Hayagriva. |

The small fixtures under `tests/fixtures` include examples adapted from the Typst Hayagriva, BibLaTeX, and Citationberg repositories.

## License

Refkit is licensed under either of:

- Apache License, Version 2.0, available in [LICENSE-APACHE](LICENSE-APACHE)
- MIT license, available in [LICENSE-MIT](LICENSE-MIT)

The locked core Typst crates used by this release, `hayagriva 0.10.0`, `biblatex 0.12.0`, and `citationberg 0.7.0`, are also licensed as `MIT OR Apache-2.0`. Refkit uses the same dual-license expression so its source license matches those core dependency terms.
