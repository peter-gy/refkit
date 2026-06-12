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

The expressions run inside the Polars engine. Each row is treated as one BibTeX or BibLaTeX source. Row-level parse failures return null for value expressions and a diagnostic list from `diagnostics`.

Top-level functions and namespace methods use stable default output names, so multiple expressions over the same `bibtex` column can be selected without manual aliases. Use `alias` or named `select` expressions when a result column needs a different name.

String arguments name columns. Use `pl.lit(...)` for literal BibTeX sources or citation keys:

```python
keys = pl.DataFrame({"key": ["doe2024"]})
out = keys.select(citation=prk.cite(pl.lit(df["bibtex"][0]), "key"))
```

Use `cite_sequence` when one row has an ordered list of citation keys and the output should preserve citation order:

```python
batch = pl.DataFrame({"keys": [["doe2024", "roe2022"]]})
out = batch.select(citations=prk.cite_sequence(pl.lit(df["bibtex"][0]), "keys"))
```

`polars-refkit` requires Polars `>=1.41,<1.42`.

## Expressions

| Function | Return | Behavior |
| --- | --- | --- |
| `cite(bibtex, key, style="apa", locale="en-US", strict=False)` | `String` | Renders one citation as text. Missing keys and row parse failures return null. |
| `cite_html(bibtex, key, style="apa", locale="en-US", strict=False)` | `String` | Renders one citation as escaped HTML. |
| `cite_rendered(bibtex, key, style="apa", locale="en-US", strict=False)` | `Struct[text, html]` | Renders one citation with both text and HTML fields. |
| `cite_sequence(bibtex, keys, style="apa", locale="en-US", strict=False)` | `List[String]` | Renders an ordered list of citation texts from a `List[String]` key column. Missing keys and row parse failures return null for the row. |
| `cite_sequence_html(bibtex, keys, style="apa", locale="en-US", strict=False)` | `List[String]` | Renders an ordered list of citation HTML strings from a `List[String]` key column. |
| `cite_sequence_rendered(bibtex, keys, style="apa", locale="en-US", strict=False)` | `List[Struct[text, html]]` | Renders ordered citations with both text and HTML fields. |
| `bibliography_html(bibtex, style="apa", locale="en-US", strict=False, all=True)` | `String` | Renders all entries as an HTML bibliography. `all=False` returns the empty cited bibliography because Polars rows do not carry citation history. Row parse failures return null. |
| `bibliography_text(bibtex, style="apa", locale="en-US", strict=False, all=True)` | `String` | Renders all entries as a plain-text bibliography. |
| `bibliography_rendered(bibtex, style="apa", locale="en-US", strict=False, all=True)` | `Struct[text, html]` | Renders all entries with both bibliography formats. |
| `entry_count(bibtex, strict=False)` | `UInt32` | Counts normalized entries in each BibTeX string. |
| `keys(bibtex, strict=False)` | `List[String]` | Returns normalized entry keys in source order. |
| `entries(bibtex, fields=("key", "entry_type", "title", "doi", "volume"), strict=False)` | `List[Struct]` | Projects normalized entries into Polars-native rows. |
| `parse_report(bibtex, strict=False)` | `Struct[ok, entry_count, keys, diagnostics]` | Parses each row once and returns a summary struct. |
| `diagnostics(bibtex, strict=False)` | `List[String]` | Returns an empty list for valid rows and parse messages for invalid rows. |
| `entries_json(bibtex, strict=False)` | `String` | Returns normalized Hayagriva entry JSON with `id` and `key` fields. |

The `bibtex_*` function names remain available for explicit BibTeX call sites, including `bibtex_to_csl_json`. Prefer `entries_json` or `bibtex_to_hayagriva_json` when the JSON shape matters.

## Expression Namespace

The same operations are available from `pl.Expr.refkit`.

```python
out = df.select(
    keys=pl.col("bibtex").refkit.keys(),
    count=pl.col("bibtex").refkit.entry_count(),
    citation=pl.col("bibtex").refkit.cite(pl.col("key")),
    citations=pl.col("bibtex").refkit.cite_sequence(pl.col("keys")),
    entries=pl.col("bibtex").refkit.entries(),
)
```

Namespace methods return expressions with names that match the method, such as `keys`, `entry_count`, and `cite`. Name outputs in `select`, `with_columns`, or `alias` when a call site needs a different column name.

`entries` returns a list of structs. Explode and unnest it to query entries as rows:

```python
entries = (
    df.select(entries=pl.col("bibtex").refkit.entries())
    .explode("entries")
    .unnest("entries")
)
```

## Scope

Use `polars-refkit` when BibTeX source lives in a dataframe and the result should stay in a Polars query plan. Use `refkit` for raw BibTeX repair with comments, preambles, string definitions, failed blocks, ordering, and source spans.

## Development

```bash
uv sync --all-packages --group benchmark
uv run maturin develop --manifest-path packages/polars-refkit/Cargo.toml
uv run pytest packages/polars-refkit/tests --no-cov
```

## License

`polars-refkit` is licensed under the Apache License, Version 2.0, available in [LICENSE-APACHE](LICENSE-APACHE).
