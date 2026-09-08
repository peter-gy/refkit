# Polars RefKit

Parse, inspect, render, and format BibTeX columns inside [Polars](https://pola.rs/) eager and lazy queries. Each source row owns an independent bibliography and citation state.

[Documentation](https://peter-gy.github.io/refkit/guides/polars) · [Expression reference](https://peter-gy.github.io/refkit/reference/polars) · [Python objects](https://pypi.org/project/refkit/)

## Render a citation column

Install in Python 3.10 through 3.14 with Polars 1.29 or newer:

```bash
python -m pip install polars-refkit
```

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
print(result.to_dicts())
```

```text
[{'citation': '(Doe, 2024)', 'entries': 1}]
```

Importing `polars_refkit` registers `pl.Expr.refkit`. A string argument names a column. Use `pl.lit(...)` for literal bibliography text or citation keys.

## Choose an operation

| Task | Expressions |
| --- | --- |
| Parse and inspect | `entry_count`, `keys`, `entries`, `can_parse`, `diagnostics`, `has_diagnostics`, `parse_report` |
| Render citations | `cite`, `cite_each`, `cite_group` |
| Render every entry | `full_bibliography` |
| Inspect render failures | `render_report` |
| Format and deduplicate | `tidy_bibtex`, `tidy_bibtex_report` |

Choose `output="text"`, `"html"`, or `"rendered"` on rendering expressions. Pass formatting settings as an `options=` dictionary. Value expressions return null on row-local failures. Reports retain structured diagnostics, errors, or rename records beside the source row.

## Use with agents

The package includes version-matched task guidance:

```python
import polars_refkit.agent

help(polars_refkit.agent)
```

[Agent integration](https://peter-gy.github.io/refkit/reference/agent-docs) covers discovery. [Data shapes](https://peter-gy.github.io/refkit/reference/data-shapes) defines reports and [Pyodide](https://peter-gy.github.io/refkit/pyodide) records the tested browser runtime.

RefKit is alpha software. Public APIs may change before 1.0.

[Apache-2.0 license](https://github.com/peter-gy/refkit/blob/main/LICENSE)
