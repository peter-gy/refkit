from __future__ import annotations

from scripts.architecture_contract import ROOT, _dependency_names, check_contract


def test_repository_matches_the_architecture_contract() -> None:
    assert check_contract(ROOT) == []


def test_dependency_names_include_target_specific_dependencies() -> None:
    manifest = {
        "dependencies": {"serde": {}},
        "target": {"cfg(unix)": {"dependencies": {"pyo3": {}}}},
    }

    assert _dependency_names(manifest) == {"pyo3", "serde"}
