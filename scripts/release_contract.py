from __future__ import annotations

import argparse
import re
import sys
import tomllib
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
REPOSITORY = "https://github.com/peter-gy/refkit"
NUMERIC_VERSION_COMPONENT = r"(?:0|[1-9][0-9]*)"
RELEASE_TAG = re.compile(
    rf"^v(?P<version>{NUMERIC_VERSION_COMPONENT}\."
    rf"{NUMERIC_VERSION_COMPONENT}\."
    rf"{NUMERIC_VERSION_COMPONENT}(?:-rc\.{NUMERIC_VERSION_COMPONENT})?)$"
)


class ReleaseContractError(ValueError):
    """Raised when publishable package metadata is inconsistent."""


def _read_toml(root: Path, relative_path: str) -> dict[str, Any]:
    return tomllib.loads((root / relative_path).read_text(encoding="utf-8"))


def _project_version(root: Path, relative_path: str) -> str:
    return str(_read_toml(root, relative_path)["project"]["version"])


def _package_version(root: Path, relative_path: str) -> str:
    return str(_read_toml(root, relative_path)["package"]["version"])


def _workspace_version(root: Path) -> str:
    return str(_read_toml(root, "Cargo.toml")["workspace"]["package"]["version"])


def _tag_version(tag: str) -> str:
    match = RELEASE_TAG.fullmatch(tag)
    if match is None:
        raise ReleaseContractError(f"release tag {tag!r} must match vX.Y.Z or vX.Y.Z-rc.N")
    return match.group("version")


def validate_release_contract(root: Path = ROOT, tag: str | None = None) -> str:
    canonical = _workspace_version(root)
    expected = _tag_version(tag) if tag is not None else canonical
    versions = {
        "rust workspace": canonical,
        "bibtex-tidy-rs": _package_version(root, "crates/bibtex-tidy-rs/Cargo.toml"),
        "python workspace": _project_version(root, "pyproject.toml"),
        "refkit": _project_version(root, "packages/refkit/pyproject.toml"),
        "refkit-core": _project_version(root, "packages/refkit-core/pyproject.toml"),
        "polars-refkit": _project_version(root, "packages/polars-refkit/pyproject.toml"),
        "polars-refkit Rust crate": _package_version(
            root, "packages/polars-refkit/rust/Cargo.toml"
        ),
    }

    mismatches = [
        f"{name} has {version}, expected {expected}"
        for name, version in versions.items()
        if version != expected
    ]

    refkit = _read_toml(root, "packages/refkit/pyproject.toml")
    expected_core_dependency = f"refkit-core=={expected}"
    if expected_core_dependency not in refkit["project"].get("dependencies", []):
        mismatches.append(f"refkit must depend on {expected_core_dependency}")

    cargo_workspace = _read_toml(root, "Cargo.toml")
    rust_core = cargo_workspace["workspace"]["dependencies"]["refkit-core"]
    if not isinstance(rust_core, dict) or rust_core.get("version") != expected:
        mismatches.append(f"Rust workspace must depend on refkit-core {expected}")

    tidy_manifest = _read_toml(root, "crates/bibtex-tidy-rs/Cargo.toml")
    tidy_core = tidy_manifest["dependencies"]["refkit-core"]
    if (
        not isinstance(tidy_core, dict)
        or tidy_core.get("version") != expected
        or tidy_core.get("path") != "../refkit-core"
    ):
        mismatches.append(f"bibtex-tidy-rs must depend on refkit-core {expected}")

    rust_repositories = {
        "Rust workspace": cargo_workspace["workspace"]["package"].get("repository"),
        "bibtex-tidy-rs": tidy_manifest["package"].get("repository"),
        "polars-refkit Rust crate": _read_toml(root, "packages/polars-refkit/rust/Cargo.toml")[
            "package"
        ].get("repository"),
    }
    for name, repository in rust_repositories.items():
        if repository != REPOSITORY:
            mismatches.append(f"{name} repository must be {REPOSITORY}")

    python_projects = {
        "refkit": "packages/refkit/pyproject.toml",
        "refkit-core": "packages/refkit-core/pyproject.toml",
        "polars-refkit": "packages/polars-refkit/pyproject.toml",
    }
    for name, relative_path in python_projects.items():
        project = _read_toml(root, relative_path)["project"]
        if project.get("urls", {}).get("Repository") != REPOSITORY:
            mismatches.append(f"{name} project.urls.Repository must be {REPOSITORY}")

    if mismatches:
        raise ReleaseContractError("\n".join(mismatches))
    return expected


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate version alignment for RefKit release artifacts."
    )
    parser.add_argument("--tag", help="Release tag in vX.Y.Z or vX.Y.Z-rc.N form")
    parser.add_argument("--root", type=Path, default=ROOT, help=argparse.SUPPRESS)
    args = parser.parse_args(argv)

    try:
        version = validate_release_contract(args.root.resolve(), args.tag)
    except (KeyError, OSError, ReleaseContractError, tomllib.TOMLDecodeError) as error:
        sys.stderr.write(f"release contract failed:\n{error}\n")
        return 1

    sys.stdout.write(f"{version}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
