from __future__ import annotations

import pytest

import refkit as rk
from refkit.types import DuplicateReport, MergePlan

SOURCE = (
    "@string{press={Example Press}}\n"
    "@book{a,title={First},doi={10.1234/work}}\n"
    "@book{b,title={Second},doi={10.1234/work},publisher=press,url={https://example.org/a%2Fb}}\n"
    "@misc{child,crossref={b}}"
)


def test_duplicate_review_and_merge_plan_preserve_source_expressions() -> None:
    original = rk.BibDocument.parse(SOURCE)
    report: DuplicateReport = original.find_duplicates(rules=["doi"])
    assert len(report["groups"]) == 1
    group = report["groups"][0]
    assert [member["key"] for member in group["members"]] == ["a", "b"]
    assert group["conflicts"][0]["field"] == "title"
    unresolved: MergePlan = original.plan_merge([0, 1], retain=0)
    assert unresolved["patch"] is None
    plan = original.plan_merge(
        [0, 1], retain=0, fields=[{"kind": "take", "name": "title", "entry_id": 1, "field_id": 0}]
    )
    patch = plan["patch"]
    assert patch is not None
    updated = original.apply_patch(patch)["document"]
    assert original.to_bibtex() == SOURCE
    assert updated.entries.occurrence_keys() == ["a", "child"]
    assert updated.resolve()[0]["fields"]["publisher"] == "Example Press"
    assert updated.resolve()[0]["fields"]["url"] == "https://example.org/a%2Fb"
    assert updated.resolve()[1]["fields"]["crossref"] == "a"
    assert "publisher = press" in updated.to_bibtex()


def test_merge_errors_and_empty_review_selection_are_explicit() -> None:
    document = rk.BibDocument.parse(SOURCE)
    assert document.find_duplicates(rules=[])["groups"] == []
    with pytest.raises(rk.MergeError) as failure:
        document.plan_merge([0, 0], retain=0)
    assert failure.value.code == "invalid_selection"
    with pytest.raises(rk.MergeError) as choice:
        document.plan_merge(
            [0, 1],
            retain=0,
            fields=[{"kind": "take", "name": "title", "entry_id": 2, "field_id": 0}],
        )
    assert choice.value.code == "invalid_choice"


def test_expression_patch_input_preserves_macros_and_rejects_extra_fields() -> None:
    document = rk.BibDocument.parse("@string{press={Press}}@book{a,title={A}}")
    updated = document.apply_patch(
        [
            {
                "kind": "add_field",
                "entry_id": 0,
                "name": "publisher",
                "value": "press # { Extra}",
                "expression": True,
            }
        ]
    )["document"]
    assert updated.resolve()[0]["fields"]["publisher"] == "Press Extra"
    with pytest.raises(rk.PatchError):
        document.apply_patch(
            [
                {
                    "kind": "set_field",
                    "entry_id": 0,
                    "field_id": 0,
                    "value": "{A}, note={injected}",
                    "expression": True,
                }
            ]
        )
