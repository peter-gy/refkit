# Render bibliography columns

`cite` renders one key per row. `cite_each` renders an ordered key list with shared citation state within the row. `cite_group` combines a key list into one citation. `full_bibliography` renders every entry in each source row. Set `output="text"`, `"html"`, or `"rendered"` for strings or `{text, html}` structs.

```python
import polars as pl
import polars_refkit

source = "@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}"
frame = pl.DataFrame(
    {
        "source": [source, source, "@article{broken,", None],
        "key": ["doe2024", "missing", "doe2024", "doe2024"],
        "keys": [["doe2024"], ["missing"], ["doe2024"], ["doe2024"]],
    }
)
result = frame.select(
    citation=pl.col("source").refkit.cite("key", output="text"),
    report=pl.col("source").refkit.render_report("keys"),
)
preview = result.head(20).to_dicts()

assert preview[0]["citation"] == "(Doe, 2024)"
assert preview[0]["report"]["ok"] is True
assert preview[1]["citation"] is None
assert preview[1]["report"]["error_code"] == "missing_key"
assert preview[2]["report"]["error_code"] == "parse_error"
assert preview[3]["report"] is None
```

A string such as `"key"` names a column. Use `pl.lit("doe2024")` for a literal citation key. `render_report` distinguishes missing keys, parsing failures, and rendering failures while preserving parse diagnostics. Resolve missing keys before accepting the complete output.
