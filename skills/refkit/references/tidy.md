# Format BibTeX

`tidy_bibtex` returns canonical source, warnings, the parsed entry count, and key rename records. `TidyOptions` controls formatting and duplicate handling.

```python
import refkit as rk

source = """@article{same, title={First}}
@book{same, title={Second}}
"""
result = rk.tidy_bibtex(source, options=rk.TidyOptions(duplicates=["key"], sort_fields=True))
preview = result.bibtex
warnings = [{"code": item.code, "message": item.message} for item in result.warnings]
renames = result.renames

assert result.count == 2
assert any(item["code"] == "duplicate_entry" for item in warnings)
assert renames == []
assert "First" in preview and "Second" in preview
```

Inspect warnings before accepting duplicate merges. `renames` identifies each changed entry occurrence with `entry_id`, `old_key`, and `new_key`. Use those records to review downstream citation changes when generating keys. A duplicate old key needs occurrence-level resolution.

`tidy_file(path)` reads and formats a file. Supply `output=intended_path` to write the result. `TidySyntaxError` identifies malformed input through line, column, byte, character, and message fields.

Generate a key with an explicit template and inspect the occurrence-level change:

```python
import refkit as rk

source = "@article{old, author={Doe, Jane}, title={Work}, year={2024}}"
result = rk.tidy_bibtex(source, options=rk.TidyOptions(generate_keys="[auth:lower][year]"))
assert result.renames == [{"entry_id": 0, "old_key": "old", "new_key": "doe2024"}]
assert rk.Library.parse_bibtex(result.bibtex).keys() == ["doe2024"]
```
