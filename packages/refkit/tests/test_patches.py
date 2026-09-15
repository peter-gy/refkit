from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor

import pytest

import refkit as rk
from refkit.types import BibPatch, BibPatchResult


def test_atomic_patch_returns_new_snapshot_mappings_and_byte_changes() -> None:
    source = "% é\n@book{a,title={Old},note={Remove}}\n@misc{b,title={B}}"
    original = rk.BibDocument.parse(source)
    entry = original.entries["a"]
    field = entry.fields["title"]
    patch: BibPatch = [
        {"kind": "set_field", "entry_id": entry.id, "field_id": field.id, "value": "Updated"},
        {"kind": "remove_field", "entry_id": entry.id, "field_id": entry.fields["note"].id},
        {"kind": "add_field", "entry_id": entry.id, "name": "doi", "value": "10.1234/a"},
        {"kind": "rename_entry", "entry_id": entry.id, "key": "renamed"},
        {"kind": "set_entry_type", "entry_id": entry.id, "entry_type": "article"},
        {"kind": "remove_entry", "entry_id": original.entries["b"].id},
        {"kind": "add_entry", "before": entry.id, "key": "new", "entry_type": "book"},
    ]
    result: BibPatchResult = original.apply_patch(patch)
    updated = result["document"]
    assert original.to_bibtex() == source
    assert field.value == "Old"
    assert updated.entries.occurrence_keys() == ["new", "renamed"]
    assert updated.entries["renamed"].fields["title"].value == "Updated"
    after = result["entries"][0]["after"]
    assert after is not None and after["id"] == 1
    assert result["entries"][0]["fields"][1]["after"] is None
    assert result["entries"][1]["after"] is None
    original_bytes = source.encode()
    updated_bytes = updated.to_bibtex().encode()
    before_cursor = after_cursor = 0
    for change in result["changes"]:
        assert isinstance(change["before"], tuple)
        assert (
            original_bytes[before_cursor : change["before"][0]]
            == updated_bytes[after_cursor : change["after"][0]]
        )
        before_cursor, after_cursor = change["before"][1], change["after"][1]
    assert original_bytes[before_cursor:] == updated_bytes[after_cursor:]
    with ThreadPoolExecutor(max_workers=1) as pool:
        assert pool.submit(lambda: (field.value, updated.to_bibtex())).result() == (
            "Old",
            updated.to_bibtex(),
        )


def test_patch_errors_leave_snapshot_and_handles_unchanged() -> None:
    original = rk.BibDocument.parse("@book{a,title={Old}}")
    field = original.entries["a"].fields["title"]
    patch: BibPatch = [
        {"kind": "set_field", "entry_id": field.entry_id, "field_id": field.id, "value": "New"}
    ]
    with pytest.raises(rk.PatchError) as failure:
        original.apply_patch(patch + patch)
    assert failure.value.code == "overlap"
    assert failure.value.operation == 1
    assert field.value == "Old"
    with pytest.raises(rk.PatchError) as missing:
        original.apply_patch([{"kind": "remove_entry", "entry_id": 100}])
    assert missing.value.code == "invalid_target"
    assert original.to_bibtex() == "@book{a,title={Old}}"


def test_patch_reference_rewrites_use_final_unambiguous_targets() -> None:
    original = rk.BibDocument.parse("@book{a,title={A}}@misc{child,crossref={a}}")
    result = original.apply_patch([{"kind": "rename_entry", "entry_id": 0, "key": "renamed"}])
    assert result["document"].resolve()[1]["fields"]["crossref"] == "renamed"
    assert any(
        change["kind"] == "rewrite_reference" and not change["operations"]
        for change in result["changes"]
    )
    with pytest.raises(rk.PatchError) as failure:
        original.apply_patch(
            [
                {"kind": "rename_entry", "entry_id": 0, "key": "renamed"},
                {"kind": "add_entry", "key": "renamed", "entry_type": "book"},
            ]
        )
    assert failure.value.code == "ambiguous_reference"
