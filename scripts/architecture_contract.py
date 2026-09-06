from __future__ import annotations

import argparse
import re
import sys
import tomllib
from pathlib import Path
from typing import Any

ROOT = Path(__file__).parents[1]
PORTABLE_CORE = Path("crates/refkit-core/Cargo.toml")
NATIVE_ADAPTER = Path("packages/refkit/rust/Cargo.toml")
POLARS_ADAPTER = Path("packages/polars-refkit/rust/Cargo.toml")
REFKIT_PROJECT = Path("packages/refkit/pyproject.toml")
NATIVE_PACKAGES = (REFKIT_PROJECT, Path("packages/polars-refkit/pyproject.toml"))
RELEASED_CORE_DEPENDENCIES = ("biblatex", "hayagriva")
REFKIT_RUNTIME_DEPENDENCIES = ["agent-plugins==0.2.0"]
CARGO_LOCKS = (
    Path("Cargo.lock"),
    Path("packages/polars-refkit/rust/Cargo.lock"),
)
ALLOWED_DEPENDENCIES = {
    PORTABLE_CORE: {
        "dependencies": {"biblatex", "hayagriva", "indexmap"},
        "dev-dependencies": {"serde", "serde_yaml", "walkdir"},
    },
    NATIVE_ADAPTER: {
        "dependencies": {"pyo3", "refkit-core", "serde_json"},
        "dev-dependencies": {"pyo3"},
    },
    POLARS_ADAPTER: {
        "dependencies": {
            "polars",
            "polars-core",
            "pyo3",
            "pyo3-polars",
            "refkit-core",
            "serde",
        },
        "dev-dependencies": set(),
    },
}
FORBIDDEN_CORE_SOURCE = {
    "host standard library": re.compile(
        r"\b(?:std\s*::\s*(?:env|fs|net|path|process)|"
        r"use\s+std\s*::\s*\{[^}]*\b(?:env|fs|net|path|process)\b)",
        re.DOTALL,
    ),
    "adapter dependency": re.compile(r"\b(?:pyo3|polars(?:_core)?)\b"),
    "upstream type in a public declaration": re.compile(
        r"^\s*pub(?:\([^)]*\))?\s+(?:fn|struct|enum|type|trait|use)\b[^\n]*"
        r"\b(?:hayagriva|biblatex|serde_json)\b",
        re.MULTILINE,
    ),
}


def _load(path: Path) -> dict[str, Any]:
    with path.open("rb") as file:
        return tomllib.load(file)


def _dependency_names(value: dict[str, Any]) -> set[str]:
    names = set()
    for key, child in value.items():
        if key in {"dependencies", "dev-dependencies", "build-dependencies"}:
            names.update(child)
        elif isinstance(child, dict):
            names.update(_dependency_names(child))
    return names


def _dependency_errors(
    manifest_path: Path,
    manifest: dict[str, Any],
    allowed: dict[str, set[str]],
) -> list[str]:
    errors = []
    for section, allowed_names in allowed.items():
        unexpected = set(manifest.get(section, {})) - allowed_names
        if unexpected:
            errors.append(
                f"{manifest_path} contains unclassified {section}: " + ", ".join(sorted(unexpected))
            )
    return errors


def _refkit_dependency_errors(manifest: dict[str, Any]) -> list[str]:
    dependencies = manifest.get("project", {}).get("dependencies", [])
    if dependencies == REFKIT_RUNTIME_DEPENDENCIES:
        return []
    return [
        "packages/refkit runtime dependencies must contain only "
        f"{', '.join(REFKIT_RUNTIME_DEPENDENCIES)}"
    ]


def _core_source_errors(root: Path) -> list[str]:
    errors = []
    source_root = root / PORTABLE_CORE.parent / "src"
    for path in sorted(source_root.rglob("*.rs")):
        source = path.read_text(encoding="utf-8")
        errors.extend(
            f"{path.relative_to(root)} reaches a host boundary through {boundary}"
            for boundary, pattern in FORBIDDEN_CORE_SOURCE.items()
            if pattern.search(source)
        )
    return errors


def _released_dependency_errors(
    core: dict[str, Any], locks: dict[Path, dict[str, Any]]
) -> list[str]:
    errors = []
    expected_versions = {}
    dependencies = core.get("dependencies", {})

    for name in RELEASED_CORE_DEPENDENCIES:
        dependency = dependencies.get(name)
        version = dependency.get("version") if isinstance(dependency, dict) else None
        if not isinstance(version, str) or not version.startswith("=") or len(version) == 1:
            errors.append(f"portable core must pin {name} to one exact release with =<version>")
            continue
        expected_versions[name] = version[1:]

    for lock_path, lock in locks.items():
        packages = lock.get("package", [])
        for name, expected in expected_versions.items():
            resolved = [package for package in packages if package.get("name") == name]
            if len(resolved) != 1:
                errors.append(f"{lock_path} must resolve exactly one {name} package")
                continue

            package = resolved[0]
            if package.get("version") != expected:
                errors.append(f"{lock_path} must resolve {name} {expected}")
            source = package.get("source")
            if not isinstance(source, str) or not source.startswith("registry+"):
                errors.append(f"{lock_path} must resolve {name} from a registry release")

    return errors


def check_contract(root: Path) -> list[str]:
    workspace = _load(root / "Cargo.toml")
    core = _load(root / PORTABLE_CORE)
    native = _load(root / NATIVE_ADAPTER)
    polars = _load(root / POLARS_ADAPTER)
    errors = []

    manifests = {PORTABLE_CORE: core, NATIVE_ADAPTER: native, POLARS_ADAPTER: polars}
    for manifest_path, allowed in ALLOWED_DEPENDENCIES.items():
        errors.extend(_dependency_errors(manifest_path, manifests[manifest_path], allowed))
    errors.extend(_core_source_errors(root))

    locks = {relative_path: _load(root / relative_path) for relative_path in CARGO_LOCKS}
    errors.extend(_released_dependency_errors(core, locks))

    members = set(workspace["workspace"]["members"])
    expected_members = {"crates/refkit-core", "packages/refkit/rust"}
    if members != expected_members:
        errors.append("root workspace must contain the portable core and refkit native adapter")
    if "packages/polars-refkit/rust" in members:
        errors.append("Polars adapter must remain in its package-local Cargo workspace")

    native_dependencies = native.get("dependencies", {})
    if not native_dependencies.get("pyo3", {}).get("workspace"):
        errors.append("native adapter must use the workspace PyO3 dependency")
    if not native_dependencies.get("refkit-core", {}).get("workspace"):
        errors.append("native adapter must use the workspace portable core dependency")

    if "workspace" not in polars:
        errors.append("Polars adapter must declare its package-local workspace")
    polars_core = polars.get("dependencies", {}).get("refkit-core", {})
    core_path = (root / POLARS_ADAPTER.parent / polars_core.get("path", "")).resolve()
    if core_path != (root / PORTABLE_CORE.parent).resolve():
        errors.append("Polars adapter must depend on the shared portable core")

    refkit = _load(root / REFKIT_PROJECT)
    errors.extend(_refkit_dependency_errors(refkit))

    for relative_path in NATIVE_PACKAGES:
        pyproject = _load(root / relative_path)
        maturin = pyproject.get("tool", {}).get("maturin", {})
        if maturin.get("locked") is not True:
            errors.append(f"{relative_path} must enable locked Maturin builds")
        excludes = maturin.get("exclude", [])
        errors.extend(
            f"{relative_path} must exclude {generated} artifacts"
            for generated in ("__pycache__", ".pyc", ".pyo")
            if not any(generated in pattern for pattern in excludes)
        )

    return errors


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Validate refkit workspace ownership boundaries.")
    parser.add_argument(
        "--root",
        type=Path,
        default=ROOT,
        help="Repository root. Defaults to the checkout containing this script.",
    )
    return parser.parse_args()


def main() -> None:
    root = _parse_args().root.resolve()
    errors = check_contract(root)
    if errors:
        raise SystemExit("Architecture contract failed:\n" + "\n".join(f"- {e}" for e in errors))
    sys.stdout.write("Architecture contract passed\n")


if __name__ == "__main__":
    main()
