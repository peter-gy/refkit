from __future__ import annotations

import re
import sys
from pathlib import Path
from typing import Any

from scripts.architecture_contract import ROOT, _load

WORKSPACES = (
    Path("Cargo.toml"),
    Path("packages/refkit-js/rust/Cargo.toml"),
    Path("packages/polars-refkit/rust/Cargo.toml"),
)
MEMBERS = (
    Path("crates/refkit-core/Cargo.toml"),
    Path("packages/refkit/rust/Cargo.toml"),
    *WORKSPACES[1:],
)
RUST_DENIES = {
    "unsafe_code",
    "unused_must_use",
    "unused_imports",
    "unused_variables",
    "dead_code",
    "unfulfilled_lint_expectations",
    "missing_docs",
}
CLIPPY_DENIES = {
    "allow_attributes",
    "allow_attributes_without_reason",
    "dbg_macro",
    "todo",
    "unimplemented",
    "unwrap_used",
    "expect_used",
    "panic",
    "print_stdout",
    "print_stderr",
    "wildcard_imports",
    "undocumented_unsafe_blocks",
    "multiple_unsafe_ops_per_block",
    "excessive_nesting",
    "indexing_slicing",
    "clone_on_ref_ptr",
    "redundant_clone",
    "needless_pass_by_value",
    "large_stack_arrays",
    "large_futures",
    "missing_errors_doc",
    "missing_panics_doc",
}
ATTRIBUTES = re.compile(r'^[ \t]*#\s*(!?)\s*\[((?:[^"\]]|"(?:\\.|[^"\\])*")*)\]', re.MULTILINE)
STRINGS = re.compile(r'"(?:\\.|[^"\\])*"')
REASON = re.compile(r'\breason\s*=\s*"((?:\\.|[^"\\])*)"')
BROAD_EXPECT = re.compile(
    r"\b(?:clippy\s*::\s*(?:all|pedantic|restriction|nursery|allow_attributes|"
    r"allow_attributes_without_reason)|warnings|unused|dead_code)\s*[,)]"
)


def suppression_errors(source: str) -> list[str]:
    """Check suppression attributes in rustfmt-formatted first-party source."""
    errors = []
    for attribute in ATTRIBUTES.finditer(source):
        body = attribute.group(2)
        code = STRINGS.sub('""', body)
        line = source.count("\n", 0, attribute.start()) + 1
        if re.search(r"\ballow\s*\(", code):
            errors.append(f"line {line}: allow attributes are forbidden")
        if not re.search(r"\bexpect\s*\(", code):
            continue
        if attribute.group(1):
            errors.append(f"line {line}: crate-wide expectations are forbidden")
        if BROAD_EXPECT.search(code):
            errors.append(f"line {line}: broad or policy-disabling expectations are forbidden")
        reason = REASON.search(body)
        if reason is None or not reason.group(1).strip():
            errors.append(
                f"line {line}: expectation requires an invariant or false-positive reason"
            )
    return errors


def level(value: Any) -> Any:
    return value.get("level") if isinstance(value, dict) else value


def audit_policy_errors(config: dict[str, Any]) -> list[str]:
    """Require strict graph checks and reviewable, version-specific exceptions."""
    errors = []
    required = {
        "graph": {"all-features": True},
        "advisories": {"yanked": "deny", "unused-ignored-advisory": "deny"},
        "licenses": {"unused-allowed-license": "deny"},
        "bans": {"multiple-versions": "deny", "wildcards": "deny"},
        "sources": {"unknown-registry": "deny", "unknown-git": "deny"},
    }
    for section, settings in required.items():
        for name, expected in settings.items():
            if config.get(section, {}).get(name) != expected:
                errors.append(f"{section}.{name} must be {expected!r}")
    for section, field, identity in (
        ("advisories", "ignore", "id"),
        ("bans", "skip", "crate"),
    ):
        for exception in config.get(section, {}).get(field, []):
            reason = exception.get("reason") if isinstance(exception, dict) else None
            if not isinstance(reason, str) or not reason.strip():
                errors.append(f"{section}.{field} requires an individual reason")
                continue
            name = exception.get(identity, "")
            if identity == "id" and name not in {"RUSTSEC-2024-0436", "RUSTSEC-2025-0141"}:
                errors.append(f"{name}: only reviewed maintenance advisories may be ignored")
            if identity == "crate" and re.fullmatch(r"[a-zA-Z0-9_-]+@=\d+\.\d+\.\d+", name) is None:
                errors.append(f"{name}: duplicate exception requires an exact crate version")
    if config.get("bans", {}).get("skip-tree"):
        errors.append("bans.skip-tree hides dependency subgraphs")
    return errors


def validate(root: Path = ROOT) -> list[str]:
    errors = []
    workspace = _load(root / WORKSPACES[0]).get("workspace", {})
    canonical = workspace.get("lints", {})
    floor = workspace.get("package", {}).get("rust-version")
    if not isinstance(floor, str) or not floor:
        errors.append("Cargo.toml: a shared Rust build floor is required")
    errors.extend(
        f"Cargo.toml: select individual {group} lints, not the whole group"
        for group in ("nursery", "restriction")
        if group in canonical.get("clippy", {})
    )
    for group, names in (("rust", RUST_DENIES), ("clippy", CLIPPY_DENIES)):
        errors.extend(
            f"Cargo.toml: {group}.{name} must be deny"
            for name in sorted(names)
            if level(canonical.get(group, {}).get(name)) != "deny"
        )
    errors.extend(
        f"Cargo.toml: clippy.{group} must deny at priority -1"
        for group in ("all", "pedantic")
        if canonical.get("clippy", {}).get(group) != {"level": "deny", "priority": -1}
    )
    errors.extend(
        f"{manifest}: workspace lint policy differs from Cargo.toml"
        for manifest in WORKSPACES[1:]
        if _load(root / manifest).get("workspace", {}).get("lints") != canonical
    )
    for manifest in WORKSPACES:
        audit = manifest.parent / "deny.toml"
        if not (root / audit).is_file():
            errors.append(f"{audit}: dependency audit policy is required")
            continue
        errors.extend(f"{audit}: {error}" for error in audit_policy_errors(_load(root / audit)))
    for manifest in MEMBERS:
        package = _load(root / manifest)
        declared = package.get("package", {}).get("rust-version")
        if declared != {"workspace": True} and declared != floor:
            errors.append(f"{manifest}: Rust build floor differs from Cargo.toml")
        if package.get("lints") != {"workspace": True}:
            errors.append(f"{manifest}: package must inherit workspace lints")
        if manifest in (
            Path("packages/refkit/rust/Cargo.toml"),
            Path("packages/polars-refkit/rust/Cargo.toml"),
        ):
            if package.get("features") != {"default": [], "abi3": ["pyo3/abi3-py310"]}:
                errors.append(
                    f"{manifest}: use abi3 without the deprecated extension-module feature"
                )
            project_path = root / manifest.parent.parent / "pyproject.toml"
            project = _load(project_path) if project_path.is_file() else {}
            if project.get("tool", {}).get("maturin", {}).get("features") != ["abi3"]:
                errors.append(f"{manifest}: Maturin must select the abi3 feature")
        for directory in ("src", "tests", "examples", "benches"):
            for source in (root / manifest.parent / directory).rglob("*.rs"):
                errors.extend(
                    f"{source.relative_to(root)}: {error}"
                    for error in suppression_errors(source.read_text())
                )
    config = _load(root / "clippy.toml")
    if config.get("msrv") != floor:
        errors.append("clippy.toml: MSRV differs from Cargo.toml")
    for name, expected in (("too-many-lines-threshold", 100), ("excessive-nesting-threshold", 4)):
        if config.get(name) != expected:
            errors.append(f"clippy.toml: {name} must be {expected}")
    return errors


def main() -> None:
    errors = validate()
    if errors:
        raise SystemExit("\n".join(errors))
    sys.stdout.write("Rust quality contract passed\n")


if __name__ == "__main__":
    main()
