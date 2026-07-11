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
