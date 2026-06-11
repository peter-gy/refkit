# polars-refkit

`polars-refkit` adds Rust-backed Polars expressions for BibTeX columns. It imports as `polars_refkit`.

```python
import polars as pl
import polars_refkit as prk

df = pl.DataFrame(
    {
        "bibtex": [
            "@article{doe2024, title={Fast Citations}, year={2024}}",
        ],
        "key": ["doe2024"],
    }
)

out = df.select(
    citation=prk.cite_bibtex("bibtex", "key"),
    bibliography=prk.bibliography_bibtex("bibtex"),
    count=prk.bibtex_entry_count("bibtex"),
    keys=prk.bibtex_keys("bibtex"),
)
```

The expressions run inside the Polars engine. Each row is treated as one clean BibTeX or BibLaTeX source. Row-level parse failures return null for value expressions and a diagnostic list from `bibtex_diagnostics`.

`polars-refkit` requires Polars `>=1.41,<1.42`.

## Expressions

| Function | Return | Behavior |
| --- | --- | --- |
| `cite_bibtex(bibtex, key, style="apa", locale="en-US", strict=False)` | `String` | Renders one citation as text. Missing keys and row parse failures return null. |
| `bibliography_bibtex(bibtex, style="apa", locale="en-US", all=True, strict=False)` | `String` | Renders one bibliography as HTML. Row parse failures return null. |
| `bibtex_entry_count(bibtex, strict=False)` | `UInt32` | Counts normalized entries in each BibTeX string. |
| `bibtex_keys(bibtex, strict=False)` | `List[String]` | Returns normalized entry keys in source order. |
| `bibtex_diagnostics(bibtex)` | `List[String]` | Returns an empty list for valid rows and parse messages for invalid rows. |
| `bibtex_to_csl_json(bibtex, strict=False)` | `String` | Returns normalized Hayagriva entry JSON with `id` and `key` fields. |

Each function accepts a column name, a `polars.Expr`, or a literal expression source.

## Expression Namespace

The same operations are available from `pl.Expr.refkit`.

```python
out = df.select(
    pl.col("bibtex").refkit.keys(),
    pl.col("bibtex").refkit.entry_count(),
    pl.col("bibtex").refkit.cite(pl.col("key")),
)
```

Namespace methods alias their output to short names such as `keys`, `entry_count`, `citation`, `bibliography`, `diagnostics`, and `to_csl_json`.

## Scope

Use `polars-refkit` when BibTeX source lives in a dataframe and the result should stay in a Polars query plan. Use `refkit` for raw BibTeX repair with comments, preambles, string definitions, failed blocks, ordering, and source spans.

## Development

```bash
uv sync --all-packages --group benchmark
uv run maturin develop --manifest-path packages/polars-refkit/Cargo.toml
uv run pytest packages/polars-refkit/tests --no-cov
```

## License

`polars-refkit` is licensed under either of:

- Apache License, Version 2.0, available in [LICENSE-APACHE](LICENSE-APACHE)
- MIT license, available in [LICENSE-MIT](LICENSE-MIT)
