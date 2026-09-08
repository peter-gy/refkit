from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

from scripts import artifact_manifest
from scripts.artifact_manifest import check, record


def _artifact_set(root: Path) -> None:
    artifacts = {
        "cpython_Linux": "refkit-1.2.3-cp310-abi3-manylinux_2_17_x86_64.whl",
        "cpython_macOS": "refkit-1.2.3-cp310-abi3-macosx_11_0_arm64.whl",
        "cpython_Windows": "refkit-1.2.3-cp310-abi3-win_amd64.whl",
        "pyemscripten_3.14": "refkit-1.2.3-cp314-cp314-pyemscripten_2026_0_wasm32.whl",
        "sdist": "refkit-1.2.3.tar.gz",
    }
    for target, filename in artifacts.items():
        archive = root / filename
        archive.write_bytes(filename.encode())
        manifest = {
            "schema": 1,
            "package": "refkit",
            "artifact": f"refkit_{target}",
            "source": "a" * 40,
            "tools": {
                "python": "3.14.0",
                "rustc": "rustc 1.95.0" if target != "sdist" else None,
                "rust_toolchain": "stable" if target != "sdist" else None,
                "uv": "uv 0.11.0",
                "build_constraints": [
                    "maturin==1.15.0",
                    "agent-plugins==0.2.0",
                    "hatchling==1.32.0",
                ],
            },
            "files": {filename: hashlib.sha256(archive.read_bytes()).hexdigest()},
        }
        (root / f"refkit_{target}.json").write_text(json.dumps(manifest))


def test_release_artifacts_match_recorded_builds(tmp_path: Path) -> None:
    _artifact_set(tmp_path)

    assert check(tmp_path, "refkit", "1.2.3", "a" * 40) == []


@pytest.mark.parametrize("tags", ["cp311-abi3", "cp310-cp310"])
def test_cpython_artifacts_require_python310_stable_abi(tmp_path: Path, tags: str) -> None:
    _artifact_set(tmp_path)
    manifest = tmp_path / "refkit_cpython_Linux.json"
    document = json.loads(manifest.read_text())
    original, digest = next(iter(document["files"].items()))
    renamed = original.replace("cp310-abi3", tags)
    (tmp_path / original).rename(tmp_path / renamed)
    document["files"] = {renamed: digest}
    manifest.write_text(json.dumps(document))

    assert check(tmp_path, "refkit", "1.2.3", "a" * 40) == [
        f"{renamed}: CPython wheels must use the Python 3.10 stable ABI"
    ]


@pytest.mark.parametrize("failure", ["content", "missing", "source", "version"])
def test_release_artifacts_reject_drift(tmp_path: Path, failure: str) -> None:
    _artifact_set(tmp_path)
    if failure == "content":
        (tmp_path / "refkit-1.2.3.tar.gz").write_bytes(b"changed")
    elif failure == "missing":
        (tmp_path / "refkit_sdist.json").unlink()
    version = "2.0.0" if failure == "version" else "1.2.3"
    source = "b" * 40 if failure == "source" else "a" * 40

    errors = check(tmp_path, "refkit", version, source)

    expected = {
        "content": "SHA-256 mismatch",
        "missing": "must cover",
        "source": "identity mismatch",
        "version": "expected refkit 2.0.0",
    }
    assert any(expected[failure] in error for error in errors)


def test_wheel_provenance_records_selected_compiler(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    wheel = tmp_path / "refkit-1.2.3-cp314-cp314-pyemscripten_2026_0_wasm32.whl"
    wheel.write_bytes(b"built wheel")
    monkeypatch.setenv("GITHUB_SHA", "a" * 40)

    def command_version(command: list[str]) -> str:
        if command == ["rustup", "run", "1.93.0", "rustc", "--version"]:
            return "rustc 1.93.0"
        if command == ["uv", "--version"]:
            return "uv 0.11.0"
        raise AssertionError(f"Unexpected build tool: {command}")

    monkeypatch.setattr(artifact_manifest, "_version", command_version)
    manifest = record(tmp_path, "refkit", "refkit_pyemscripten_3.14", "1.93.0")
    document = json.loads(manifest.read_text())

    assert document["source"] == "a" * 40
    assert document["tools"]["rustc"] == "rustc 1.93.0"
    assert document["files"] == {wheel.name: hashlib.sha256(wheel.read_bytes()).hexdigest()}
