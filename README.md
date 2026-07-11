# refkit

`refkit` parses bibliography files, renders CSL citations, formats and edits raw BibTeX, and applies Rust-backed bibliography capabilities to Polars columns.

## Install

```bash
pip install refkit
pip install polars-refkit
```

`refkit` is a pure Python package with an exact dependency on `refkit-core`.
`refkit-core` contains the Rust/PyO3 extension as `refkit_core._refkit_core`.
Installing `refkit` also installs the matching native wheel for the current Python platform.

RefKit supports Python 3.11 through 3.14. The [PyEmscripten wheels](docs/pyodide.md) target Pyodide 314.0.2.

## Render Citations From Python

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

`Library` is the normalized citation database. `Style` loads a bundled or explicit CSL style. `Document.render` renders the whole citation document at once and returns `RenderedDocument` with named citations and a cited bibliography. Each rendered citation and bibliography is `Rendered` with `text`, `html`, and `tree`.
`Cite` names one citation item. `CitationGroup` renders several items as one citation. `Citation(id, group)` gives that rendered citation a stable lookup name. Citation ids must be unique inside one `Document.render` call.

Use the one-call helpers for scripts:

```python
rk.cite("refs.bib", "doe2024", style="ieee").text
rk.full_bibliography("refs.bib", style="chicago-author-date").html
```

## Format BibTeX From Python

`tidy_bibtex` formats BibTeX text and returns `TidyResult` with the formatted source, warnings, and entry count.

```python
import refkit as rk

result = rk.tidy_bibtex(
    """
@ARTICLE {doe2024,
  pages={6-13},
  year={2024},}
"""
)

print(result.bibtex)
print(result.count)
```

Use `TidyOptions` for formatting choices:

```python
options = rk.TidyOptions(sort_fields=True, wrap=88)
tidied = rk.BibDocument.read("refs.bib").tidy(options=options)
```

## Process BibTeX Columns In Polars

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
    literal_citation=pl.col("bibtex").refkit.cite(pl.lit("doe2024")),
    each_citation=pl.col("bibtex").refkit.cite_each(pl.col("keys")),
    grouped_citation=pl.col("bibtex").refkit.cite_group(pl.col("keys")),
    bibliography=pl.col("bibtex").refkit.full_bibliography_html(),
    formatted=pl.col("bibtex").refkit.tidy_bibtex(sort_fields=True),
    count=pl.col("bibtex").refkit.entry_count(),
    keys=pl.col("bibtex").refkit.keys(),
    entries=pl.col("bibtex").refkit.entries(),
)
```

`polars-refkit` runs inside the Polars expression engine. Each row is one BibTeX or BibLaTeX source. Strings name columns. Use `pl.lit(...)` for literal BibTeX or citation keys. Parse or formatting failures return null for value expressions. `can_parse` returns whether a row can produce a normalized library, and `has_diagnostics`, `diagnostics`, `parse_report`, and `tidy_bibtex_report` expose row messages.

## Choose A Package

| Package | Entry point | Use it for |
| --- | --- | --- |
| `refkit` | `import refkit as rk` | Citation rendering, normalized library access, selectors, and raw BibTeX editing. |
| `refkit-core` | Installed by `refkit` | Native Rust/PyO3 implementation used by `refkit`, including Pyodide-compatible wheels. |
| `polars-refkit` | `import polars_refkit as prk` | BibTeX parsing, formatting, inspection, and rendering inside eager or lazy Polars plans. |

## Capabilities

| Capability | `refkit` | `polars-refkit` |
| --- | --- | --- |
| Read normalized bibliography data | `Library.read`, `Library.parse_bibtex`, `Library.parse_yaml` | `entry_count`, `can_parse`, `has_diagnostics`, `keys`, `entries`, `parse_report` |
| Render citations | `Document.render`, `Citation`, `Cite`, `CitationGroup`, `cite` | `cite`, `cite_html`, `cite_rendered`, `cite_each`, `cite_group` |
| Render bibliographies | `Document.cited_bibliography`, `Document.full_bibliography`, `full_bibliography` | `full_bibliography_text`, `full_bibliography_html`, `full_bibliography_rendered` |
| Load CSL styles and locales | `Style.load`, `Style.from_path`, `Style.from_xml`, `Locale.load` | `style=` and `locale=` arguments |
| Inspect entries | Mapping access, selectors, `project`, `to_dicts` | `keys`, `entries`, `to_hayagriva_json` |
| Format and edit raw BibTeX | `tidy_bibtex`, `tidy_file`, `BibDocument` | `tidy_bibtex`, `tidy_bibtex_report` |
| Export rendered output | `Rendered.text`, `Rendered.html`, `Rendered.tree` | string and struct expressions |

## Documentation

- [refkit Python API](packages/refkit/README.md)
- [refkit-core native package](packages/refkit-core/README.md)
- [polars-refkit expressions](packages/polars-refkit/README.md)
- [API contracts](docs/api-contracts.md)
- [Use RefKit in Pyodide](docs/pyodide.md)
- [Migration guide](docs/migration.md)

## Contributing

The [developer documentation](development_docs/README.md) covers architecture, setup, testing, packaging, releases, and benchmarks.

## License

The packages are licensed under the Apache License, Version 2.0. See [NOTICE](NOTICE) for upstream citation and bibliography component acknowledgements.
