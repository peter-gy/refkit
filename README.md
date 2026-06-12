# refkit workspace

This workspace contains two Python packages backed by Rust:

| Package | Import | Purpose |
| --- | --- | --- |
| `refkit` | `import refkit as rk` | Citation parsing, CSL rendering, normalized library access, and raw BibTeX editing. |
| `polars-refkit` | `import polars_refkit as prk` | Polars expressions for BibTeX columns. |

Both packages are versioned as `0.0.1`. `refkit` supports CPython 3.11 through 3.14, and the wheels use the Python 3.11 stable ABI.

## refkit

```python
import refkit as rk

library = rk.Library.read("refs.bib")
style = rk.Style.load("apa")
doc = rk.Document(library, style, locale="en-US")

print(doc.cite("doe2024").text)
print(doc.bibliography().html)
```

`refkit` supports:

- `Library.read` and `Library.parse` for BibTeX, BibLaTeX, and Hayagriva YAML
- `Style.load`, `Style.from_path`, and `Style.from_xml`
- `Document.cite` and `Document.bibliography`
- `Rendered.text`, `Rendered.html`, and `Rendered.tree`
- `BibDocument` for raw BibTeX comments, preambles, strings, failed blocks, order, spans, and field edits

See [packages/refkit/README.md](packages/refkit/README.md) for the package contract.

## polars-refkit

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

`polars-refkit` supports:

- citation and bibliography rendering as text, HTML, or `{text, html}` structs
- ordered citation batches from `List[String]` key columns with `cite_sequence`
- entry counts, key lists, parse reports, diagnostics, and validity checks
- normalized entry projection as `List[Struct]`
- Hayagriva entry JSON export for interchange
- the `pl.Expr.refkit` namespace

See [packages/polars-refkit/README.md](packages/polars-refkit/README.md) for the package contract.

## Development

```bash
uv sync --all-packages --group benchmark
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
make package-check
```

## License

The packages are licensed under the Apache License, Version 2.0.
