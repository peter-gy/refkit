from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

import pytest

from scripts.release_contract import ReleaseContractError, validate_release_contract

ROOT = Path(__file__).resolve().parents[2]
CONTRACT_FILES = (
    "Cargo.toml",
    "pyproject.toml",
    "crates/bibtex-tidy-rs/Cargo.toml",
    "packages/polars-refkit/pyproject.toml",
    "packages/polars-refkit/rust/Cargo.toml",
    "packages/refkit/pyproject.toml",
    "packages/refkit-core/pyproject.toml",
)


def copy_contract_files(destination: Path) -> None:
    for relative_path in CONTRACT_FILES:
        target = destination / relative_path
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / relative_path, target)


def test_release_contract_matches_workspace_and_tag() -> None:
    version = validate_release_contract(ROOT)

    assert validate_release_contract(ROOT, f"v{version}") == version


@pytest.mark.parametrize(
    ("version", "tag"),
    [
        ("1.2.3", "v1.2.3"),
        ("1.2.3-rc.4", "v1.2.3-rc.4"),
    ],
)
def test_release_contract_accepts_supported_release_tag(
    tmp_path: Path,
    version: str,
    tag: str,
) -> None:
    copy_contract_files(tmp_path)
    current_version = validate_release_contract(tmp_path)
    for relative_path in CONTRACT_FILES:
        path = tmp_path / relative_path
        source = path.read_text(encoding="utf-8")
        path.write_text(source.replace(current_version, version), encoding="utf-8")

    assert validate_release_contract(tmp_path, tag) == version


@pytest.mark.parametrize(
    "tag",
    [
        "1.2.3",
        "v01.2.3",
        "v1.02.3",
        "v1.2.03",
        "v1.2.3-beta.1",
        "v1.2.3-rc1",
        "v1.2.3-rc.01",
    ],
)
def test_release_contract_rejects_unsupported_release_tags(tag: str) -> None:
    with pytest.raises(
        ReleaseContractError,
        match=r"must match vX\.Y\.Z or vX\.Y\.Z-rc\.N",
    ):
        validate_release_contract(ROOT, tag)


def test_release_contract_reports_core_dependency_drift(tmp_path: Path) -> None:
    copy_contract_files(tmp_path)
    version = validate_release_contract(tmp_path)
    manifest = tmp_path / "packages/refkit/pyproject.toml"
    source = manifest.read_text(encoding="utf-8")
    manifest.write_text(
        source.replace(
            f'"refkit-core=={version}"',
            '"refkit-core==9.9.9"',
        ),
        encoding="utf-8",
    )

    with pytest.raises(ReleaseContractError, match=f"must depend on refkit-core=={version}"):
        validate_release_contract(tmp_path)


@pytest.mark.parametrize(
    ("relative_path", "message"),
    [
        ("Cargo.toml", "Rust workspace repository must be"),
        (
            "packages/refkit-core/pyproject.toml",
            r"refkit-core project\.urls\.Repository must be",
        ),
    ],
)
def test_release_contract_reports_repository_drift(
    tmp_path: Path,
    relative_path: str,
    message: str,
) -> None:
    copy_contract_files(tmp_path)
    manifest = tmp_path / relative_path
    source = manifest.read_text(encoding="utf-8")
    manifest.write_text(
        source.replace(
            "https://github.com/peter-gy/refkit",
            "https://example.invalid/refkit",
            1,
        ),
        encoding="utf-8",
    )

    with pytest.raises(ReleaseContractError, match=message):
        validate_release_contract(tmp_path)


def test_release_contract_command_prints_validated_version() -> None:
    version = validate_release_contract(ROOT)
    result = subprocess.run(
        [
            sys.executable,
            str(ROOT / "scripts/release_contract.py"),
            "--tag",
            f"v{version}",
        ],
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 0
    assert result.stdout == f"{version}\n"
    assert result.stderr == ""
