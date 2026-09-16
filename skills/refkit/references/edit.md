# Edit BibTeX

Use `BibDocument.apply_patch` to create an edited snapshot while preserving unrelated source bytes. Entry keys are case-sensitive. Field lookup is case-insensitive. Duplicate entry and field occurrences require explicit selection.

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
result = raw.apply_patch(
    [
        {
            "kind": "set_field",
            "entry_id": fields[0].entry_id,
            "field_id": fields[0].id,
            "value": "Corrected title",
        }
    ]
)
updated = result["document"]
preview = updated.to_bibtex()
assert fields[0].value == "Second title"

assert (
    preview
    == """% Keep this comment.
@article{doe2024, TITLE={First title}, year={2024}}
@article{doe2024, tItLe={Corrected title}, year={2025}}
"""
)
```

Review `preview`, `result["changes"]`, occurrence mappings in `result["entries"]`, and `result["warnings"]`. Call `updated.write(intended_path)` when the task authorizes that write. Patch kinds are `set_field`, `add_field`, `remove_field`, `add_entry`, `remove_entry`, `rename_entry`, and `set_entry_type`. Use input-snapshot IDs and final values for explicitly edited reference fields. `PatchError.code` and `.operation` explain an atomic rejection. Inspect source diagnostics and failed blocks before planning repairs. Review rename changes before updating citations outside the bibliography.

Use `expression=True` when a patch value is one complete BibTeX value expression, including its delimiters or macros. Extra assignments are rejected.

For duplicate review, call `raw.find_duplicates(rules=["doi"])` and inspect each group's `evidence` and `conflicts`. Matching signatures are candidate evidence. Call `raw.plan_merge(entry_ids, retain=retained_id, fields=choices)` to select field occurrences with `{"kind": "take", "name": name, "entry_id": entry_id, "field_id": field_id}` or omit a field with `{"kind": "drop", "name": name}`. A null `plan["patch"]` requires more choices. Inspect a non-null patch before calling `raw.apply_patch(plan["patch"])`. Plans preserve source expressions and update unambiguous references. `MergeError.code` explains invalid selections or unsafe reference transformations.
