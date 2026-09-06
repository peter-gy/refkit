<p align="center">
  <a href="https://peter-gy.github.io/refkit/">
    <img alt="RefKit" src="https://peter-gy.github.io/refkit/brand/refkit-lockup-horizontal-light-transparent.svg" width="420">
  </a>
</p>

<p align="center">
  BibTeX expressions for eager and lazy Polars queries.
</p>

<p align="center">
  <a href="https://peter-gy.github.io/refkit/guides/polars"><strong>Documentation</strong></a> ·
  <a href="https://github.com/peter-gy/refkit"><strong>Source</strong></a> ·
  <a href="https://pypi.org/project/refkit/"><strong>Python objects</strong></a>
</p>

<p align="center">
  <a href="https://pypi.org/project/polars-refkit/"><img alt="PyPI version" src="https://img.shields.io/pypi/v/polars-refkit.svg"></a>
  <a href="https://pypi.org/project/polars-refkit/"><img alt="Supported Python versions" src="https://img.shields.io/pypi/pyversions/polars-refkit.svg"></a>
  <a href="https://github.com/peter-gy/refkit/actions/workflows/ci.yml"><img alt="CI status" src="https://github.com/peter-gy/refkit/actions/workflows/ci.yml/badge.svg?branch=main"></a>
  <a href="https://github.com/peter-gy/refkit/blob/main/LICENSE"><img alt="Apache-2.0 license" src="https://img.shields.io/pypi/l/polars-refkit.svg"></a>
</p>

> **Alpha:** Public APIs may change before 1.0.

`polars-refkit` parses, inspects, renders, and formats bibliography source
inside [Polars](https://pola.rs/) expressions. Each bibliography source row is
an independent normalized library and render boundary.

## Run a citation expression

Add `polars-refkit` to a Python 3.11 through 3.14 environment with Polars 1.29
or newer:

```bash
python -m pip install polars-refkit
```

Importing `polars_refkit` registers the `pl.Expr.refkit` namespace:

```python
import polars as pl
import polars_refkit

frame = pl.DataFrame(
    {
        "bibtex": ["@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}"],
        "key": ["doe2024"],
    }
)

result = frame.select(
    citation=pl.col("bibtex").refkit.cite("key"),
    entries=pl.col("bibtex").refkit.entry_count(),
)
print(result)
```

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

A plain string argument names a column. Wrap literal bibliography source or
citation keys with `pl.lit(...)`.

## Choose an expression

| Job | Expressions |
| --- | --- |
| Parse and inspect | `entry_count`, `can_parse`, `has_diagnostics`, `keys`, `entries`, `diagnostics`, `parse_report` |
| Render one citation | `cite`, `cite_html`, `cite_rendered` |
| Render key lists | `cite_each*`, `cite_group*` |
| Render every entry | `full_bibliography_text`, `full_bibliography_html`, `full_bibliography_rendered` |
| Format source | `tidy_bibtex`, `tidy_bibtex_report` |

Value expressions map row-local null input, parse failure, missing citation
keys, and render failure to null. Report expressions expose parser or formatter
details. Invalid dtypes, styles, projections, and broadcasting lengths raise
when the query executes.

## Documentation

- [Polars guide](https://peter-gy.github.io/refkit/guides/polars) covers eager, lazy, literal, column, and recovery workflows.
- [Expression reference](https://peter-gy.github.io/refkit/reference/polars) lists every expression, dtype, default, and failure mode.
- [Data shapes](https://peter-gy.github.io/refkit/reference/data-shapes) records projection rows and report structs.
- [Tidy options](https://peter-gy.github.io/refkit/reference/tidy-options) defines canonical formatting arguments.
- [Pyodide compatibility](https://peter-gy.github.io/refkit/pyodide) records the tested WebAssembly runtime family.

## License

`polars-refkit` is licensed under the
[Apache License 2.0](https://github.com/peter-gy/refkit/blob/main/LICENSE).
[NOTICE](https://github.com/peter-gy/refkit/blob/main/NOTICE) records upstream
citation and bibliography components.
