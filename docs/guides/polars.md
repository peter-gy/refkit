---
description: Parse, inspect, render, format, broadcast, and diagnose bibliography source inside Polars queries.
---

# Process Polars Columns

`polars-refkit` applies bibliography capabilities inside Polars queries. Each bibliography source row forms an independent normalized library and render boundary.

## Install and register the namespace

```bash
python -m pip install polars-refkit
```

```python
import polars as pl
import polars_refkit
```

Importing `polars_refkit` registers `pl.Expr.refkit`. The same operations are also exported as top-level functions from `polars_refkit`.

## Parse and inspect rows

```python
frame = pl.DataFrame(
    {
        "bibtex": [
            "@article{doe2024, title={Fast Citations}, year={2024}}"
        ],
        "key": ["doe2024"],
    }
)

result = frame.select(
    count=pl.col("bibtex").refkit.entry_count(),
    keys=pl.col("bibtex").refkit.keys(),
    entries=pl.col("bibtex").refkit.entries(
        fields=["key", "entry_type", "title"]
    ),
)
```

A plain string argument names a column. Use `pl.lit(...)` for literal bibliography source or citation keys.

## Render citations

```python
result = frame.select(
    citation=pl.col("bibtex").refkit.cite("key"),
    citation_html=pl.col("bibtex").refkit.cite_html("key"),
    rendered=pl.col("bibtex").refkit.cite_rendered("key"),
)
```

Choose the citation shape from the key input:

| Operation | Key value | Result per row |
| --- | --- | --- |
| `cite*` | `String` | One citation. |
| `cite_each*` | `List[String]` | One separate citation per key, in order. |
| `cite_group*` | `List[String]` | One grouped citation containing the ordered keys. |

The `*` families provide text, HTML, and `{text, html}` rendered variants. `full_bibliography_*` renders every normalized entry in the row.

## Broadcast a Singleton Input

Citation operations accept equal-length inputs or a length-one input on either side. A singleton valid bibliography source is parsed once within that expression and reused for every key row.

```python
result = pl.DataFrame({"key": ["doe2024", "roe2022"]}).select(
    pl.lit(source).refkit.cite("key")
)
```

Other unequal lengths raise a Polars `ComputeError` when the query executes.

## Handle row failures

Value and render expressions map null inputs, parse failures, missing citation keys, and rendering failures to null rows. `can_parse`, `diagnostics`, and `parse_report` expose parser outcomes:

```python
result = frame.select(
    ok=pl.col("bibtex").refkit.can_parse(recovery="report"),
    diagnostics=pl.col("bibtex").refkit.diagnostics(recovery="report"),
    report=pl.col("bibtex").refkit.parse_report(recovery="report"),
)
```

`recovery="report"` keeps recoverable normalized entries and their parser diagnostics. The `parse_report` expression performs one parse for its complete struct result.

Static option errors are raised while the expression is constructed. Invalid input dtypes, unknown styles, unsupported projection fields, duplicate output names, and broadcasting failures raise when an eager query runs or a lazy plan collects.

## Format rows

```python
result = frame.select(
    bibtex=pl.col("bibtex").refkit.tidy_bibtex(sort_fields=True, wrap=88),
    report=pl.col("bibtex").refkit.tidy_bibtex_report(sort_fields=True),
)
```

The report contains `ok`, `bibtex`, `count`, `warnings`, and `error`. Read [Polars Expressions](/reference/polars) for exact dtypes, nullability, defaults, and empty-list behavior.

## Use lazy plans

Every operation returns `pl.Expr` and works in lazy plans:

```python
result = (
    frame.lazy()
    .select(pl.col("bibtex").refkit.entry_count().alias("entries"))
    .collect()
)
```

Separate expressions parse independently. Name or alias repeated operations such as two `cite` expressions because their default output names are identical.
