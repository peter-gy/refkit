from __future__ import annotations

import argparse
import sys
import tomllib
from pathlib import Path
from typing import Any

ROOT = Path(__file__).parents[1]
PORTABLE_CORE = Path("crates/refkit-core/Cargo.toml")
NATIVE_ADAPTER = Path("packages/refkit-core/rust/Cargo.toml")
POLARS_ADAPTER = Path("packages/polars-refkit/rust/Cargo.toml")
TIDY_ADAPTER = Path("crates/bibtex-tidy-rs/Cargo.toml")
FORBIDDEN_CORE_DEPENDENCIES = {"polars", "polars-core", "pyo3", "pyo3-polars"}
NATIVE_PACKAGES = ("packages/refkit-core/pyproject.toml", "packages/polars-refkit/pyproject.toml")


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


def check_contract(root: Path) -> list[str]:
    workspace = _load(root / "Cargo.toml")
    core = _load(root / PORTABLE_CORE)
    native = _load(root / NATIVE_ADAPTER)
    polars = _load(root / POLARS_ADAPTER)
    tidy = _load(root / TIDY_ADAPTER)
    errors = []

    core_dependencies = _dependency_names(core)
    forbidden = core_dependencies & FORBIDDEN_CORE_DEPENDENCIES
    if forbidden:
        errors.append(
            "portable core contains adapter dependencies: " + ", ".join(sorted(forbidden))
        )

    members = set(workspace["workspace"]["members"])
    if "crates/refkit-core" not in members or "packages/refkit-core/rust" not in members:
        errors.append("root workspace must contain the portable core and native adapter")
    if "packages/polars-refkit/rust" in members:
        errors.append("Polars adapter must remain in its package-local Cargo workspace")
    if "crates/bibtex-tidy-rs" in members:
        errors.append("bibtex-tidy-rs must remain in its package-local Cargo workspace")

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

    if "workspace" not in tidy:
        errors.append("bibtex-tidy-rs must declare its package-local workspace")
    tidy_core = tidy.get("dependencies", {}).get("refkit-core", {})
    tidy_core_path = (root / TIDY_ADAPTER.parent / tidy_core.get("path", "")).resolve()
    if tidy_core_path != (root / PORTABLE_CORE.parent).resolve():
        errors.append("bibtex-tidy-rs must depend on the shared portable core")

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
