from pathlib import Path

import pytest

from scripts import rust_quality_contract as quality


@pytest.mark.parametrize(
    "attribute",
    [
        "#[allow(dead_code)]",
        '#![allow(clippy::all, reason = "needed")]',
        '#![expect(clippy::indexing_slicing, reason = "bounded input")]',
        '#[expect(clippy::all, reason = "bounded input")]',
        '#[expect(dead_code, reason = "reserved helper")]',
        "#[expect(clippy::indexing_slicing)]",
        '#[expect(clippy::indexing_slicing, reason = " ")]',
        "#[cfg_attr(test, allow(dead_code))]",
    ],
)
def test_rejects_unreviewable_suppressions(attribute: str) -> None:
    assert quality.suppression_errors(attribute + "\nfn sample() {}")


def test_accepts_local_invariant_and_ignores_documented_attribute_text() -> None:
    source = """/// Avoid `#[allow(dead_code)]`.
#[expect(
    clippy::indexing_slicing,
    reason = "The input cursor is produced by char_indices; allow appears only in this reason."
)]
fn cursor() {}
"""
    assert quality.suppression_errors(source) == []


@pytest.mark.parametrize(
    "section, field, exception",
    [
        ("advisories", "ignore", "RUSTSEC-2024-0436"),
        ("advisories", "ignore", {"id": "RUSTSEC-2026-0176", "reason": "Upgrade deferred"}),
        ("bans", "skip", {"crate": "syn", "reason": "Different major APIs"}),
        ("bans", "skip", {"crate": "syn@=2.0.119", "reason": " "}),
    ],
)
def test_audit_exceptions_require_bounded_review(
    section: str, field: str, exception: object
) -> None:
    config = quality._load(quality.ROOT / "deny.toml")
    config[section][field] = [exception]
    assert quality.audit_policy_errors(config)


def test_rejects_weakened_workspace_and_member_policies(tmp_path: Path) -> None:
    for manifest in quality.WORKSPACES:
        target = tmp_path / manifest
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text('[workspace.lints.rust]\nunsafe_code = "allow"\n')
    for manifest in quality.MEMBERS:
        target = tmp_path / manifest
        if not target.exists():
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text("[lints]\nworkspace = false\n")
    (tmp_path / "clippy.toml").write_text("too-many-lines-threshold = 1000\n")
    errors = quality.validate(tmp_path)
    assert any("unsafe_code must be deny" in error for error in errors)
    assert any("inherit workspace lints" in error for error in errors)
    assert any("threshold must be 100" in error for error in errors)
