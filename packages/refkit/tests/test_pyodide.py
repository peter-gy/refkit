from __future__ import annotations

import importlib
import sys
from importlib import metadata
from pathlib import Path
from types import ModuleType
from typing import Any, cast

import pytest

ROOT = Path(__file__).resolve()
REFKIT_SRC = str(ROOT.parents[1] / "src")
CORE_SRC = str(ROOT.parents[2] / "refkit-core-py" / "src")

CORE_EXPORTS = (
    "BibDocument",
    "BibEntry",
    "BibEntryMap",
    "BibField",
    "BibFieldMap",
    "Citation",
    "CitationGroup",
    "Cite",
    "Document",
    "Entry",
    "Library",
    "Locale",
    "MissingReferenceError",
    "RefkitError",
    "Rendered",
    "RenderedDocument",
    "Style",
)


def _fake_core_module(name: str, *, version: str) -> ModuleType:
    module = ModuleType(name)
    dynamic_module = cast(Any, module)
    dynamic_module.__version__ = version
    dynamic_module.build_info = f"refkit-core {version} pyemscripten test"
    dynamic_module.build_mode = "release"
    for export in CORE_EXPORTS:
        setattr(module, export, type(export, (), {}))
    return module


def test_mock_pyodide_sets_platform_and_stub_modules(mock_pyodide: Any) -> None:
    micropip = ModuleType("micropip")

    with mock_pyodide(micropip=micropip):
        assert sys.platform == "emscripten"
        assert importlib.import_module("pyodide") is sys.modules["pyodide"]
        assert importlib.import_module("micropip") is micropip

    assert sys.modules.get("micropip") is not micropip


def test_pyodide_env_fixture_sets_platform(pyodide_env: None) -> None:
    assert sys.platform == "emscripten"


def test_mock_pyodide_can_wrap_sync_tests(mock_pyodide: Any) -> None:
    @mock_pyodide()
    def wrapped() -> str:
        return sys.platform

    assert wrapped() == "emscripten"


def test_refkit_core_import_uses_pyodide_extension_module(
    monkeypatch: pytest.MonkeyPatch,
    mock_pyodide: Any,
) -> None:
    version = metadata.version("refkit-core")
    fake_extension = _fake_core_module("refkit_core._refkit_core", version=version)

    with mock_pyodide():
        monkeypatch.delitem(sys.modules, "refkit_core", raising=False)
        monkeypatch.delitem(sys.modules, "refkit_core._refkit_core", raising=False)
        monkeypatch.setitem(sys.modules, "refkit_core._refkit_core", fake_extension)
        monkeypatch.syspath_prepend(CORE_SRC)

        imported = cast(Any, importlib.import_module("refkit_core"))

    assert imported.__version__ == version
    assert imported.build_info == cast(Any, fake_extension).build_info
    assert imported.Library is cast(Any, fake_extension).Library


def test_refkit_import_uses_public_core_package_under_pyodide(
    monkeypatch: pytest.MonkeyPatch,
    mock_pyodide: Any,
) -> None:
    version = metadata.version("refkit-core")
    fake_core = _fake_core_module("refkit_core", version=version)

    with mock_pyodide():
        monkeypatch.delitem(sys.modules, "refkit", raising=False)
        monkeypatch.setitem(sys.modules, "refkit_core", fake_core)
        monkeypatch.syspath_prepend(REFKIT_SRC)

        imported = cast(Any, importlib.import_module("refkit"))

    assert imported.check_refkit_core_version()
    assert imported.build_info == cast(Any, fake_core).build_info
    assert imported.Library is cast(Any, fake_core).Library
