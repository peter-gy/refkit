---
description: Parse, inspect, render, format, broadcast, and diagnose bibliography source inside Polars queries.
---

# Process Polars Columns

`polars-refkit` applies bibliography capabilities inside Polars queries. Each bibliography source row forms an independent normalized library and render boundary.

## Install and register the namespace

```bash
python -m pip install polars-refkit
```

Importing `polars_refkit` registers `pl.Expr.refkit`. The same operations are also exported as top-level functions from `polars_refkit`.

## Parse and inspect rows

```python
import polars as pl
import polars_refkit

frame = pl.DataFrame(
    {
        "bibtex": ["@article{doe2024, title={Fast Citations}, year={2024}}"],
        "key": ["doe2024"],
    }
)

result = frame.select(
    count=pl.col("bibtex").refkit.entry_count(),
    keys=pl.col("bibtex").refkit.keys(),
    entries=pl.col("bibtex").refkit.entries(fields=["key", "entry_type", "title"]),
)
assert result["count"].to_list() == [1]
assert result["keys"].to_list() == [["doe2024"]]
```

A plain string argument names a column. Use `pl.lit(...)` for literal bibliography source or citation keys.

## Render citations

```python
result = frame.select(
    citation=pl.col("bibtex").refkit.cite("key"),
    citation_html=pl.col("bibtex").refkit.cite("key", output="html"),
    rendered=pl.col("bibtex").refkit.cite("key", output="rendered"),
)
```

Choose the citation shape from the key input:

| Operation | Key value | Result per row |
| --- | --- | --- |
| `cite` | `String` | One citation. |
| `cite_each` | `List[String]` | One separate citation per key, in order. |
| `cite_group` | `List[String]` | One grouped citation containing the ordered keys. |

Choose `output="text"`, `"html"`, or `"rendered"` for strings or `{text, html}` structs. `full_bibliography` renders every normalized entry in the row.

## Broadcast a Singleton Input

Citation operations accept equal-length inputs or a length-one input on either side. A singleton valid bibliography source is parsed once within that expression and reused for every key row.

```python
source = "@article{doe2024, author={Doe, Jane}, year={2024}}"
result = pl.DataFrame({"key": ["doe2024", "doe2024"]}).select(pl.lit(source).refkit.cite("key"))
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

## Inspect render failures

Use a key-list column and keep the report beside the source row:

```python
result = frame.select(
    report=pl.col("bibtex").refkit.render_report(pl.concat_list("key")),
)
```

The report contains rendered citations, structured parser diagnostics, and an error code that distinguishes parse, missing-key, and render failures. Pass `grouped=True` to render the key list as one citation.

## Format rows

```python
result = frame.select(
    bibtex=pl.col("bibtex").refkit.tidy_bibtex(options={"sort_fields": True, "wrap": 88}),
    report=pl.col("bibtex").refkit.tidy_bibtex_report(options={"sort_fields": True}),
)
```

The report contains `ok`, `bibtex`, `count`, `warnings`, `renames`, and `error`. Read [Polars Expressions](/reference/polars) for exact dtypes, nullability, defaults, and empty-list behavior.

## Use lazy plans

Every operation returns `pl.Expr` and works in lazy plans:

```python
result = frame.lazy().select(pl.col("bibtex").refkit.entry_count().alias("entries")).collect()
```

Separate expressions parse independently. Name or alias repeated operations such as two `cite` expressions because their default output names are identical.
