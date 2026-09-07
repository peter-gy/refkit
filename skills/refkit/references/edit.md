# Edit BibTeX

Use `BibDocument` to change an existing field while preserving raw source. Entry keys are case-sensitive. Field lookup is case-insensitive. Duplicate entry and field occurrences require explicit selection.

```python
import refkit as rk

source = """% Keep this comment.
@article{doe2024, TITLE={First title}, year={2024}}
@article{doe2024, tItLe={Second title}, year={2025}}
"""
raw = rk.BibDocument.parse(source)
entries = raw.entries.get_all("doe2024")
assert len(entries) == 2
fields = entries[1].fields.get_all("title")
assert len(fields) == 1
fields[0].value = "Corrected title"
preview = raw.to_bibtex()

assert (
    preview
    == """% Keep this comment.
@article{doe2024, TITLE={First title}, year={2024}}
@article{doe2024, tItLe={Corrected title}, year={2025}}
"""
)
```

Review `preview`, then call `raw.write(intended_path)` when the task authorizes that write. `BibEntry.key` is read-only. Inspect `raw.diagnostics` and `raw.failed_blocks` when the input contains malformed blocks. Use canonical formatting for generated citation keys and inspect its rename records before changing downstream citations.
