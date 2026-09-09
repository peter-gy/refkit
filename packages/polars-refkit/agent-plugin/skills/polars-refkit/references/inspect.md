# Inspect bibliography columns

Use `parse_report` for one parsing result per row, including diagnostics. Use `entries` to project selected fields. `recovery="report"` retains recoverable entries.

```python
import polars as pl
import polars_refkit

source = "@article{doe2024, title={Fast Citations}, year={2024}}"
frame = pl.DataFrame({"source": [source, "@article{broken,", None]})
result = (
    frame.lazy()
    .select(
        report=pl.col("source").refkit.parse_report(recovery="report"),
        entries=pl.col("source").refkit.entries(fields=["key", "title"], recovery="report"),
    )
    .collect()
)
preview = result.head(20).to_dicts()

assert preview[0]["report"]["ok"] is True
assert preview[0]["entries"] == [{"key": "doe2024", "title": "Fast Citations"}]
assert preview[1]["report"]["ok"] is False
assert preview[1]["report"]["diagnostics"]
assert preview[2]["report"] is None
```

Diagnostics contain `code`, `severity`, `action`, `span`, `entry`, `field`, and `message`. `span` is a struct containing UTF-8 byte offsets `start` and `end` when a source location is available. Return diagnostics beside the affected input row. Null source rows remain null.

Use `resolve` to expand macros and concatenations in every source field,
including custom fields. It returns a list of records with `key`, `entry_type`,
and `fields`. Each `fields` value is a list of `{name, value}` records. Invalid
source rows become null. Empty bibliographies become empty lists.

```python
import polars as pl
import polars_refkit

frame = pl.DataFrame({"bibtex": ["@misc{guide, custom={A {Guide}}}"]})
expanded = frame.select(pl.col("bibtex").refkit.resolve()).item()
assert expanded.to_list()[0]["fields"] == [{"name": "custom", "value": "A {Guide}"}]
```
