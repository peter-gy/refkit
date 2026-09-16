from __future__ import annotations

from pathlib import Path

import pytest

from scripts import pyodide_lock

ROOT = Path(__file__).parents[2]
LOCK_PATH = ROOT / ".github" / "pyodide" / "pylock.314.toml"


def test_pyodide_compiler_matches_the_shared_rust_floor(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setitem(pyodide_lock.RUNTIME, "rust-toolchain", "1.93.0")
    assert any(
        error.startswith("Pyodide Rust toolchain must be ")
        for error in pyodide_lock.validate_lock(LOCK_PATH)
    )


def test_pyodide_lock_rejects_a_host_platform_wheel(tmp_path: Path) -> None:
    invalid = tmp_path / "pylock.toml"
    invalid.write_text(
        LOCK_PATH.read_text().replace("py3-none-any", "cp314-cp314-macosx_11_0_arm64", 1)
    )

    errors = pyodide_lock.validate_lock(invalid)

    assert any("host-platform wheel" in error for error in errors)
    assert any("incompatible wheel identity or runtime tag" in error for error in errors)


def test_pyodide_lock_rejects_requirement_drift(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    requirements = tmp_path / "requirements.in"
    requirements.write_text("agent-plugins==0.1.0\npytest==9.1.1\n")
    monkeypatch.setattr(pyodide_lock, "REQUIREMENTS_PATH", requirements)

    errors = pyodide_lock.validate_lock(LOCK_PATH)

    assert "agent-plugins must resolve to 0.1.0" in errors


@pytest.mark.parametrize(
    ("before", "after", "message"),
    [
        ('lock-version = "1.0"', 'lock-version = "2.0"', "lock-version 1.0"),
        ('name = "iniconfig"', 'name = "agent-plugins"', "duplicate package"),
        ("https://files.pythonhosted.org/", "https://example.org/", "unsupported wheel source"),
        (
            "https://files.pythonhosted.org/",
            "http://files.pythonhosted.org/",
            "unsupported wheel source",
        ),
        ("/pyodide/v314.0.2/full/", "/pyodide/v314.0.1/full/", "must come from Pyodide"),
        (
            'sha256 = "94dd4f5d15f8688691c3628f2848fbe92d110baf96e7314c147fcc775ce45a50"',
            'sha256 = "' + "z" * 64 + '"',
            "invalid SHA-256",
        ),
    ],
)
def test_pyodide_lock_rejects_invalid_artifact_provenance(
    tmp_path: Path,
    before: str,
    after: str,
    message: str,
) -> None:
    invalid = tmp_path / "pylock.toml"
    invalid.write_text(LOCK_PATH.read_text().replace(before, after))

    assert any(message in error for error in pyodide_lock.validate_lock(invalid))


def test_pyodide_lock_requires_installable_wheel_records(tmp_path: Path) -> None:
    invalid = tmp_path / "pylock.toml"
    invalid.write_text(
        'lock-version = "1.0"\n[[packages]]\nname = "agent-plugins"\nversion = "0.2.0"\n'
    )

    assert pyodide_lock.validate_lock(invalid) == [
        "agent-plugins must resolve to exactly one wheel"
    ]


def test_pyodide_build_lock_matches_selected_runtime(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    lock = tmp_path / "uv.lock"
    lock.write_text('[[package]]\nname = "pyodide-build"\nversion = "0.34.0"\n')
    monkeypatch.setattr(pyodide_lock, "BUILD_LOCK_PATH", lock)

    assert "uv.lock must resolve pyodide-build to 0.35.1" in pyodide_lock.validate_lock(LOCK_PATH)
