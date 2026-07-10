from __future__ import annotations

from pathlib import Path

from scripts.pyodide_lock import POLARS_WHEEL_TAG, validate_lock

ROOT = Path(__file__).parents[2]
LOCK_PATH = ROOT / ".github" / "pyodide" / "pylock.314.toml"


def test_pyodide_lock_matches_the_runtime_contract() -> None:
    assert validate_lock(LOCK_PATH) == []


def test_pyodide_lock_rejects_a_host_platform_polars_wheel(tmp_path: Path) -> None:
    invalid = tmp_path / "pylock.toml"
    invalid.write_text(LOCK_PATH.read_text().replace(POLARS_WHEEL_TAG, "macosx_11_0_arm64"))

    errors = validate_lock(invalid)

    assert any("host-platform wheel" in error for error in errors)
    assert any("polars must resolve" in error for error in errors)
