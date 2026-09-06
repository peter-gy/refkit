from __future__ import annotations

from pathlib import Path
from typing import Any

from scripts.architecture_contract import (
    ROOT,
    _core_source_errors,
    _dependency_errors,
    _dependency_names,
    _released_dependency_errors,
    check_contract,
)


def test_repository_matches_the_architecture_contract() -> None:
    assert check_contract(ROOT) == []


def test_dependency_names_include_target_specific_dependencies() -> None:
    manifest = {
        "dependencies": {"serde": {}},
        "target": {"cfg(unix)": {"dependencies": {"pyo3": {}}}},
    }

    assert _dependency_names(manifest) == {"pyo3", "serde"}


def test_dependency_contract_reports_unclassified_packages() -> None:
    errors = _dependency_errors(
        Path("crates/refkit-core/Cargo.toml"),
        {"dependencies": {"serde": {}, "vendor-sdk": {}}},
        {"dependencies": {"serde"}},
    )

    assert errors == [
        "crates/refkit-core/Cargo.toml contains unclassified dependencies: vendor-sdk"
    ]


def test_portable_core_cannot_reach_host_boundaries(tmp_path: Path) -> None:
    source = tmp_path / "crates/refkit-core/src/library.rs"
    source.parent.mkdir(parents=True)
    source.write_text("use std::fs;\n", encoding="utf-8")

    assert _core_source_errors(tmp_path) == [
        "crates/refkit-core/src/library.rs reaches a host boundary through host standard library"
    ]


def test_portable_core_detects_grouped_host_imports(tmp_path: Path) -> None:
    source = tmp_path / "crates/refkit-core/src/library.rs"
    source.parent.mkdir(parents=True)
    source.write_text("use std::{fs, path::Path};\n", encoding="utf-8")

    assert _core_source_errors(tmp_path) == [
        "crates/refkit-core/src/library.rs reaches a host boundary through host standard library"
    ]


def test_released_dependencies_are_exact_and_consistent_across_locks() -> None:
    core = _core_dependencies("=0.12.0", "=0.10.1")
    locks = {
        Path("Cargo.lock"): _lock_packages("0.12.0", "0.10.1"),
        Path("adapter/Cargo.lock"): _lock_packages("0.12.0", "0.10.1"),
    }

    assert _released_dependency_errors(core, locks) == []


def test_released_dependencies_require_exact_manifest_versions() -> None:
    core = _core_dependencies("0.12.0", "=0.10.1")

    errors = _released_dependency_errors(core, {})

    assert errors == ["portable core must pin biblatex to one exact release with =<version>"]


def test_released_dependencies_reject_lock_drift_and_non_registry_sources() -> None:
    core = _core_dependencies("=0.12.0", "=0.10.1")
    locks = {
        Path("Cargo.lock"): {
            "package": [
                {
                    "name": "biblatex",
                    "version": "0.11.0",
                    "source": "registry+https://github.com/rust-lang/crates.io-index",
                },
                {
                    "name": "hayagriva",
                    "version": "0.10.1",
                    "source": "git+https://github.com/typst/hayagriva",
                },
            ]
        },
        Path("adapter/Cargo.lock"): {"package": []},
    }

    errors = _released_dependency_errors(core, locks)

    assert errors == [
        "Cargo.lock must resolve biblatex 0.12.0",
        "Cargo.lock must resolve hayagriva from a registry release",
        "adapter/Cargo.lock must resolve exactly one biblatex package",
        "adapter/Cargo.lock must resolve exactly one hayagriva package",
    ]


def _core_dependencies(biblatex: str, hayagriva: str) -> dict[str, Any]:
    return {
        "dependencies": {
            "biblatex": {"version": biblatex},
            "hayagriva": {"version": hayagriva},
        }
    }


def _lock_packages(biblatex: str, hayagriva: str) -> dict[str, Any]:
    source = "registry+https://github.com/rust-lang/crates.io-index"
    return {
        "package": [
            {"name": "biblatex", "version": biblatex, "source": source},
            {"name": "hayagriva", "version": hayagriva, "source": source},
        ]
    }
