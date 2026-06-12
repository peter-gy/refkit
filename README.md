# refkit

`refkit` parses bibliography files, renders CSL citations, edits raw BibTeX, and applies the same Rust-backed capabilities to Polars columns.

## Install

```bash
pip install refkit
pip install polars-refkit
```

Both packages are versioned as `0.0.1` and support CPython 3.11 through 3.14. Wheels use the Python 3.11 stable ABI.

## Render Citations From Python

```python
import refkit as rk

library = rk.Library.read("refs.bib")
style = rk.Style.load("apa")
doc = rk.Document(library, style, locale="en-US")

first = doc.cite("doe2024")
second = doc.cite([rk.Cite("doe2024", locator="12", label="page"), "roe2022"])

print(first.text)
print(second.text)
print(doc.bibliography().html)
```

`Library` is the normalized citation database. `Style` loads a bundled or explicit CSL style. `Document` keeps citation order, repeated citation state, and bibliography state. Each render returns `Rendered` with `text`, `html`, and `tree`.

Use the one-call helpers for scripts:

```python
rk.cite("refs.bib", "doe2024", style="ieee").text
rk.bibliography("refs.bib", style="chicago-author-date").html
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
    citation=prk.cite("bibtex", "key"),
    citations=prk.cite_sequence("bibtex", "keys"),
    bibliography=prk.bibliography_html("bibtex"),
    count=prk.entry_count("bibtex"),
    keys=prk.keys("bibtex"),
    entries=prk.entries("bibtex"),
)
```

`polars-refkit` runs inside the Polars expression engine. Each row is one BibTeX or BibLaTeX source. Parse failures return null for value expressions and diagnostics through `diagnostics` or `parse_report`.

## Choose A Package

| Package | Import | Use it for |
| --- | --- | --- |
| `refkit` | `import refkit as rk` | Citation rendering, normalized library access, selectors, and raw BibTeX editing. |
| `polars-refkit` | `import polars_refkit as prk` | BibTeX parsing, inspection, and rendering inside eager or lazy Polars plans. |
| `refkit-bench` | `python -m refkit_bench.runner` | Repository benchmark lanes for parser, renderer, raw BibTeX, and Polars workflows. |

## Capabilities

| Capability | `refkit` | `polars-refkit` |
| --- | --- | --- |
| Read normalized bibliography data | `Library.read`, `Library.parse` | `entry_count`, `keys`, `entries`, `parse_report` |
| Render citations | `Document.cite`, `cite` | `cite`, `cite_html`, `cite_rendered`, `cite_sequence` |
| Render bibliographies | `Document.bibliography`, `bibliography` | `bibliography_text`, `bibliography_html`, `bibliography_rendered` |
| Load CSL styles and locales | `Style.load`, `Style.from_path`, `Style.from_xml`, `Locale.load` | `style=` and `locale=` arguments |
| Inspect entries | Mapping access, selectors, `project`, `to_dicts` | `keys`, `entries`, `entries_json` |
| Edit raw BibTeX | `BibDocument` | Use `refkit` for block-level edits |
| Export rendered output | `Rendered.text`, `Rendered.html`, `Rendered.tree` | string and struct expressions |

## Architecture

`crates/refkit-core` owns the platform-independent work: parsing, recovery, normalized records, raw BibTeX blocks, style preparation, document rendering, rendered trees, and shared error records.

The Python packages are adapters over that core:

- `refkit` owns PyO3 classes, Python exceptions, GIL release, and Python value conversion.
- `polars-refkit` owns expression registration, dtypes, broadcasting, null mapping, eager and lazy execution, and dataframe diagnostics.

## Package Docs

- [refkit Python API](packages/refkit/README.md)
- [polars-refkit expressions](packages/polars-refkit/README.md)
- [refkit benchmark runner](benchmark/README.md)
- [API contracts](docs/api-contracts.md)
- [Migration guide](docs/migration.md)
- [Feature matrix](docs/feature-matrix.md)

## Development

```bash
uv sync --all-packages --group dev
uv run maturin develop --manifest-path packages/refkit/Cargo.toml
uv run maturin develop --manifest-path packages/polars-refkit/Cargo.toml
uv run pytest
```

Local quality gates:

```bash
make lint
make typecheck
make test
make rust
make build
```

## License

The packages are licensed under the Apache License, Version 2.0.
