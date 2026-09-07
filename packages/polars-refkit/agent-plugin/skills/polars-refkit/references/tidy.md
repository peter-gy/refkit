# Format bibliography columns

Pass a `TidyOptions` dictionary through `options=`. `tidy_bibtex` returns formatted source. `tidy_bibtex_report` returns source, warnings, occurrence count, key changes, and failure details.

```python
import polars as pl
import polars_refkit

source = "@article{same, title={First}}\n@book{same, title={Second}}"
options: polars_refkit.TidyOptions = {"duplicates": ["key"], "sort_fields": True}
frame = pl.DataFrame({"source": [source, "@article{broken,", None]})
result = frame.select(report=pl.col("source").refkit.tidy_bibtex_report(options=options))
preview = result.head(20).to_dicts()

assert preview[0]["report"]["ok"] is True
assert preview[0]["report"]["count"] == 2
assert any(warning["code"] == "duplicate_entry" for warning in preview[0]["report"]["warnings"])
assert preview[0]["report"]["renames"] == []
assert preview[1]["report"]["ok"] is False
assert preview[2]["report"] is None
```

Inspect warnings before accepting duplicate merges. Rename records identify `entry_id`, `old_key`, and `new_key`. Review them before changing downstream citations. Keep formatted source in the frame until the task calls for export to an intended path.

Generate citation keys and retain the change report in the frame:

```python
import polars as pl
import polars_refkit

source = "@article{old, author={Doe, Jane}, title={Work}, year={2024}}"
frame = pl.DataFrame({"source": [source]})
result = frame.select(
    report=pl.col("source").refkit.tidy_bibtex_report(
        options={"generate_keys": "[auth:lower][year]"},
    )
)
report = result["report"][0]
assert report["renames"] == [{"entry_id": 0, "old_key": "old", "new_key": "doe2024"}]
```
