# polars-refkit

`polars-refkit` parses, inspects, renders, and formats bibliography source inside eager and lazy Polars queries.

## Install

```bash
python -m pip install polars-refkit
```

The package supports Python 3.11 through 3.14 and Polars 1.29 or newer. Importing `polars_refkit` registers the `pl.Expr.refkit` namespace.

## Run a Citation Expression

```python
import polars as pl
import polars_refkit

frame = pl.DataFrame(
    {
        "bibtex": [
            "@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}"
        ],
        "key": ["doe2024"],
    }
)

result = frame.select(
    citation=pl.col("bibtex").refkit.cite("key"),
    entries=pl.col("bibtex").refkit.entry_count(),
)
```

Result:

```text
shape: (1, 2)
┌─────────────┬─────────┐
│ citation    ┆ entries │
│ ---         ┆ ---     │
│ str         ┆ u32     │
╞═════════════╪═════════╡
│ (Doe, 2024) ┆ 1       │
└─────────────┴─────────┘
```

A plain string argument names a column. Wrap literal bibliography source and citation keys with `pl.lit(...)`.

## Expression Groups

| Job | Expressions |
| --- | --- |
| Parse and inspect | `entry_count`, `can_parse`, `has_diagnostics`, `keys`, `entries`, `diagnostics`, `parse_report` |
| Render one citation | `cite`, `cite_html`, `cite_rendered` |
| Render key lists | `cite_each*`, `cite_group*` |
| Render every entry | `full_bibliography_text`, `full_bibliography_html`, `full_bibliography_rendered` |
| Format source | `tidy_bibtex`, `tidy_bibtex_report` |

Each bibliography source row is an independent normalized library and render boundary. Value expressions map row-local input, parse, missing-key, and render failures to null. Report expressions expose parser or formatter details. Invalid dtypes, styles, projections, and broadcasting lengths raise when the query executes.

## Documentation

- [Polars guide](https://github.com/peter-gy/refkit/blob/main/docs/guides/polars.md)
- [Expression reference](https://github.com/peter-gy/refkit/blob/main/docs/reference/polars.md)
- [Data shapes](https://github.com/peter-gy/refkit/blob/main/docs/reference/data-shapes.md)
- [Tidy options](https://github.com/peter-gy/refkit/blob/main/docs/reference/tidy-options.md)
- [Pyodide compatibility](https://github.com/peter-gy/refkit/blob/main/docs/pyodide.md)

`polars-refkit` is licensed under the Apache License 2.0.
