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
    "packages/refkit-js/package.json",
    "packages/refkit-js/package-lock.json",
    "packages/refkit-js/rust/Cargo.toml",
    "pyproject.toml",
    "crates/refkit-core/Cargo.toml",
    "packages/polars-refkit/pyproject.toml",
    "packages/polars-refkit/rust/Cargo.toml",
    "packages/refkit/pyproject.toml",
    "packages/refkit/rust/Cargo.toml",
)


def copy_contract_files(destination: Path) -> None:
    for relative_path in CONTRACT_FILES:
        target = destination / relative_path
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / relative_path, target)


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

    completed = subprocess.run(
        [
            sys.executable,
            str(ROOT / "scripts/release_contract.py"),
            "--root",
            str(tmp_path),
            "--tag",
            tag,
        ],
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr
    assert completed.stdout == f"{version}\n"


@pytest.mark.parametrize(
    "tag",
    [
        "1.2.3",
        "v01.2.3",
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


def test_release_contract_reports_native_adapter_version_drift(tmp_path: Path) -> None:
    copy_contract_files(tmp_path)
    manifest = tmp_path / "packages/refkit/rust/Cargo.toml"
    source = manifest.read_text(encoding="utf-8")
    manifest.write_text(
        source.replace("version.workspace = true", 'version = "9.9.9"', 1),
        encoding="utf-8",
    )

    with pytest.raises(ReleaseContractError, match="refkit native Rust crate has 9.9.9"):
        validate_release_contract(tmp_path)


def test_release_contract_reports_javascript_lock_drift(tmp_path: Path) -> None:
    import json

    copy_contract_files(tmp_path)
    lock = tmp_path / "packages/refkit-js/package-lock.json"
    value = json.loads(lock.read_text())
    value["packages"][""]["version"] = "9.9.9"
    lock.write_text(json.dumps(value))
    with pytest.raises(ReleaseContractError, match="package lock version"):
        validate_release_contract(tmp_path)
