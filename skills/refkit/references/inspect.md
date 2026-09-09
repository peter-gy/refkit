# Inspect bibliography data

Use `Library` for normalized lookup and projection. `recovery="report"` retains recoverable entries and records diagnostics. Strict parsing uses `recovery="error"`. Recovery raises `ParseError` when every entry fails. Inspect the exception's `.diagnostics` for structured failure details.

```python
import refkit as rk

source = """@article{doe2024, title={First title}, year={2024}}
@article{doe2024, title={Duplicate title}, year={2025}}
@book{roe2023, title={Second work}, year={2023}}
"""
library = rk.Library.parse_bibtex(source, recovery="report")
limit = 20
preview_keys = library.keys()[:limit]
rows = library.project(["key", "title", "date"], keys=preview_keys)
diagnostics = library.diagnostics
result = {
    "total": len(library),
    "entries": rows,
    "diagnostics": diagnostics[:limit],
    "diagnostic_count": len(diagnostics),
}

assert result["total"] == 2
assert rows[0]["title"] == "First title"
assert diagnostics
assert "code" in diagnostics[0] and "action" in diagnostics[0]
```

Return the entry preview, diagnostic preview, and their total counts together. Diagnostics expose `code`, `severity`, `action`, `span`, `entry`, `field`, and `message`. A span is a pair of UTF-8 byte offsets when source location is available.

Use `Library.read(path)` for files, `Library.parse_yaml(source)` for [Hayagriva bibliography YAML](https://github.com/typst/hayagriva), and `Library.select(selector)` for [Hayagriva selectors](https://github.com/typst/hayagriva#selectors). Use `project(..., keys=selected_keys)` to constrain subsequent output.

Use `BibDocument.resolve()` for every source field, including custom fields,
with string macros and concatenations expanded. It preserves TeX grouping and
escapes. Results are detached records with `key`, `entry_type`, and `fields`.
The call uses current edits and raises `ParseError` for ambiguous or invalid
source. It resolves `crossref` and `xdata` as field text. Choose `Library` for
inherited citation metadata.

```python
import refkit as rk

document = rk.BibDocument.parse("""
@string{host = {https://example.org/}}
@misc{guide, custom_link = host # {guide}, title = {A {Guide}}}
""")
entry = document.resolve()[0]
assert entry["fields"]["custom_link"] == "https://example.org/guide"
assert entry["fields"]["title"] == "A {Guide}"
```

When parsing cannot retain an entry, handle the typed failure and report its diagnostics:

```python
import refkit as rk

try:
    rk.Library.parse_bibtex("@article{broken,", recovery="report")
except rk.ParseError as error:
    diagnostics = error.diagnostics
else:
    raise AssertionError("Malformed input must report a parsing failure")

assert diagnostics
assert diagnostics[0]["action"] == "dropped_block"
```
