from __future__ import annotations

from pathlib import Path

import pytest

from scripts import pyodide_lock

ROOT = Path(__file__).parents[2]
LOCK_PATH = ROOT / ".github" / "pyodide" / "pylock.314.toml"


def test_pyodide_lock_matches_the_runtime_contract() -> None:
    assert pyodide_lock.validate_lock(LOCK_PATH) == []


def test_pyodide_lock_rejects_a_host_platform_polars_wheel(tmp_path: Path) -> None:
    invalid = tmp_path / "pylock.toml"
    invalid.write_text(
        LOCK_PATH.read_text().replace(pyodide_lock.POLARS_WHEEL_TAG, "macosx_11_0_arm64")
    )

    errors = pyodide_lock.validate_lock(invalid)

    assert any("host-platform wheel" in error for error in errors)
    assert any("polars must resolve" in error for error in errors)


def test_pyodide_lock_rejects_python_polars_abi_drift(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    requirements = tmp_path / "requirements.in"
    requirements.write_text("polars==1.32.3\npytest==9.1.1\n")
    monkeypatch.setattr(pyodide_lock, "REQUIREMENTS_PATH", requirements)

    errors = pyodide_lock.validate_lock(LOCK_PATH)

    assert "Pyodide Polars requirement must be 1.33.1 for the plugin ABI" in errors


def test_pyodide_lock_rejects_rust_plugin_abi_drift(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    cargo_lock = tmp_path / "Cargo.lock"
    cargo_lock.write_text(
        pyodide_lock.POLARS_CARGO_LOCK_PATH.read_text().replace(
            'name = "pyo3-polars"\nversion = "0.24.0"',
            'name = "pyo3-polars"\nversion = "0.23.1"',
        )
    )
    monkeypatch.setattr(pyodide_lock, "POLARS_CARGO_LOCK_PATH", cargo_lock)

    errors = pyodide_lock.validate_lock(LOCK_PATH)

    assert "pyo3-polars must resolve to 0.24.0 for the Polars plugin ABI" in errors


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
