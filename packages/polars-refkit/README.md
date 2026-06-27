# polars-refkit

`polars-refkit` adds Rust-backed Polars expressions for BibTeX and BibLaTeX columns. It imports as `polars_refkit`.

## Install

```bash
pip install polars-refkit
```

`polars-refkit` requires Polars `>=1.29` and supports CPython 3.11 through 3.14.

## Render And Inspect Rows

```python
import polars as pl
import polars_refkit as prk

df = pl.DataFrame(
    {
        "bibtex": [
            """
@article{doe2024, title={Fast Citations}, year={2024}}
@book{roe2022, title={Batch References}, year={2022}}
""",
        ],
        "key": ["doe2024"],
        "keys": [["doe2024", "roe2022"]],
    }
)

out = df.select(
    citation=pl.col("bibtex").refkit.cite(pl.col("key")),
    literal_citation=pl.col("bibtex").refkit.cite(pl.lit("doe2024")),
    each_citation=pl.col("bibtex").refkit.cite_each(pl.col("keys")),
    grouped_citation=pl.col("bibtex").refkit.cite_group(pl.col("keys")),
    bibliography=pl.col("bibtex").refkit.full_bibliography_html(),
    count=pl.col("bibtex").refkit.entry_count(),
    keys=pl.col("bibtex").refkit.keys(),
    entries=pl.col("bibtex").refkit.entries(),
)
```

Each row is one BibTeX or BibLaTeX source. The expressions run inside eager `DataFrame.select` and lazy `LazyFrame.select(...).collect()` plans. Row-level parse failures return null for value expressions and a diagnostic list from `diagnostics` or `parse_report`.

`recovery="error"` uses strict parsing. In a Polars expression, a strict row parse failure returns null for value expressions, `False` from `can_parse`, and a failed `parse_report` instead of aborting the query. `recovery="report"` keeps recoverable entries in that row and preserves parser diagnostics.

String arguments name columns. Use `pl.lit(...)` for literal BibTeX sources or citation keys:

```python
keys = pl.DataFrame({"key": ["doe2024"]})
out = keys.select(citation=pl.lit(df["bibtex"][0]).refkit.cite(pl.col("key")))
```

Use `cite_each` when one row has an ordered list of citation keys and each key should render as a separate citation:

```python
batch = pl.DataFrame({"keys": [["doe2024", "roe2022"]]})
out = batch.select(citations=pl.lit(df["bibtex"][0]).refkit.cite_each(pl.col("keys")))
```

Use `cite_group` when one row has an ordered list of citation keys and the list should render as one grouped citation:

```python
batch = pl.DataFrame({"keys": [["doe2024", "roe2022"]]})
out = batch.select(citation=pl.lit(df["bibtex"][0]).refkit.cite_group(pl.col("keys")))
```

Top-level functions and namespace methods use stable default output names, so multiple expressions over the same `bibtex` column can be selected without manual aliases. Use `alias` or named `select` expressions when a result column needs a different name.

## Capabilities

| Capability | Polars surface |
| --- | --- |
| Read normalized bibliography data | `entry_count`, `can_parse`, `has_diagnostics`, `keys`, `entries`, `parse_report`, `diagnostics` |
| Render citations | `cite`, `cite_html`, `cite_rendered`, `cite_each`, `cite_group`, and their HTML or struct variants |
| Render bibliographies | `full_bibliography_text`, `full_bibliography_html`, `full_bibliography_rendered` |
| Inspect entries | `keys`, `entries`, `to_hayagriva_json` |
| Process dataframe columns | eager `DataFrame.select` and lazy `LazyFrame.select(...).collect()` |
| Use expression namespace | `pl.Expr.refkit` methods with the same capability set |

## Expressions

| Function | Return | Behavior |
| --- | --- | --- |
| `cite(bibtex_col, key_col, style="apa", locale="en-US", recovery="error")` | `String` | Renders one citation as text. Missing keys and row parse failures return null. |
| `cite_html(bibtex_col, key_col, style="apa", locale="en-US", recovery="error")` | `String` | Renders one citation as escaped HTML. |
| `cite_rendered(bibtex_col, key_col, style="apa", locale="en-US", recovery="error")` | `Struct[text, html]` | Renders one citation with both text and HTML fields. |
| `cite_each(bibtex_col, keys_col, style="apa", locale="en-US", recovery="error")` | `List[String]` | Renders each key in a `List[String]` column as a separate citation. Missing keys and row parse failures return null for the row. |
| `cite_each_html(bibtex_col, keys_col, style="apa", locale="en-US", recovery="error")` | `List[String]` | Renders each key as separate citation HTML. |
| `cite_each_rendered(bibtex_col, keys_col, style="apa", locale="en-US", recovery="error")` | `List[Struct[text, html]]` | Renders each key as a separate citation struct. |
| `cite_group(bibtex_col, keys_col, style="apa", locale="en-US", recovery="error")` | `String` | Renders one grouped citation from a `List[String]` key column. |
| `cite_group_html(bibtex_col, keys_col, style="apa", locale="en-US", recovery="error")` | `String` | Renders one grouped citation as HTML. |
| `cite_group_rendered(bibtex_col, keys_col, style="apa", locale="en-US", recovery="error")` | `Struct[text, html]` | Renders one grouped citation with both text and HTML fields. |
| `full_bibliography_html(bibtex_col, style="apa", locale="en-US", recovery="error")` | `String` | Renders all entries in the row as an HTML bibliography. Row parse failures return null. |
| `full_bibliography_text(bibtex_col, style="apa", locale="en-US", recovery="error")` | `String` | Renders all entries in the row as plain text. |
| `full_bibliography_rendered(bibtex_col, style="apa", locale="en-US", recovery="error")` | `Struct[text, html]` | Renders all entries in the row with both bibliography formats. |
| `entry_count(bibtex_col, recovery="error")` | `UInt32` | Counts normalized entries in each BibTeX string. |
| `can_parse(bibtex_col, recovery="error")` | `Boolean` | Returns whether the row can produce a normalized library. |
| `has_diagnostics(bibtex_col, recovery="error")` | `Boolean` | Returns whether parsing produced diagnostics. |
| `keys(bibtex_col, recovery="error")` | `List[String]` | Returns normalized entry keys in source order. |
| `entries(bibtex_col, fields=("key", "title", "doi", "volume"), recovery="error")` | `List[Struct]` | Projects normalized entries into Polars-native rows. |
| `parse_report(bibtex_col, recovery="error")` | `Struct[ok, entry_count, keys, diagnostics]` | Parses each row once and returns a summary struct. |
| `diagnostics(bibtex_col, recovery="error")` | `List[String]` | Returns an empty list for valid rows and parse messages for invalid rows. |
| `to_hayagriva_json(bibtex_col, recovery="error")` | `String` | Returns normalized Hayagriva entry JSON with `id` and `key` fields. |

## Expression Namespace

The same operations are available from `pl.Expr.refkit`.

```python
out = df.select(
    keys=pl.col("bibtex").refkit.keys(),
    count=pl.col("bibtex").refkit.entry_count(),
    citation=pl.col("bibtex").refkit.cite(pl.col("key")),
    each_citation=pl.col("bibtex").refkit.cite_each(pl.col("keys")),
    grouped_citation=pl.col("bibtex").refkit.cite_group(pl.col("keys")),
    entries=pl.col("bibtex").refkit.entries(),
    hayagriva_json=pl.col("bibtex").refkit.to_hayagriva_json(),
)
```

Top-level functions and namespace methods expose one name per capability. They return expressions with names that match the method, such as `keys`, `entry_count`, `cite`, and `to_hayagriva_json`. Name outputs in `select`, `with_columns`, or `alias` when a call site needs a different column name.

Typed code can cast the namespace when the type checker does not know Polars plugin namespaces:

```python
from typing import cast

namespace = cast(prk.RefkitExprNamespace, pl.col("bibtex").refkit)
out = df.select(citation=namespace.cite(pl.col("key")))
```

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
uv sync --all-packages --group dev
(cd packages/polars-refkit && uv run maturin develop)
uv run pytest packages/polars-refkit/tests --no-cov
```

## License

`polars-refkit` is licensed under the Apache License, Version 2.0, available in [LICENSE](LICENSE). See [NOTICE](NOTICE) for upstream citation and bibliography component acknowledgements.
