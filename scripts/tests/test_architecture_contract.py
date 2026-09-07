from __future__ import annotations

import shutil
from pathlib import Path
from typing import Any

import pytest

from scripts.architecture_contract import (
    ROOT,
    _core_source_errors,
    _engine_dependency_errors,
    _refkit_dependency_errors,
    check_contract,
)


def test_repository_matches_the_architecture_contract() -> None:
    assert check_contract(ROOT) == []


def test_refkit_runtime_dependency_contract_rejects_additional_packages() -> None:
    manifest = {"project": {"dependencies": ["agent-plugins==0.2.0", "requests"]}}

    assert _refkit_dependency_errors(manifest) == [
        "packages/refkit runtime dependencies must contain only agent-plugins==0.2.0"
    ]


@pytest.fixture
def contract_root(tmp_path: Path) -> Path:
    for source in [
        ROOT / "Cargo.toml",
        ROOT / "Cargo.lock",
        *ROOT.glob("crates/*/Cargo.toml"),
        *ROOT.glob("packages/*/pyproject.toml"),
        *ROOT.glob("packages/*/rust/Cargo.toml"),
        ROOT / "packages/polars-refkit/rust/Cargo.lock",
    ]:
        destination = tmp_path / source.relative_to(ROOT)
        destination.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, destination)
    return tmp_path


@pytest.mark.parametrize(
    "section", ["dependencies", "build-dependencies", "target.'cfg(unix)'.dependencies"]
)
def test_architecture_rejects_unclassified_dependency_tables(
    contract_root: Path, section: str
) -> None:
    tmp_path = contract_root
    manifest = tmp_path / "crates/refkit-core/Cargo.toml"
    source = manifest.read_text()
    addition = 'host = { package = "pyo3", version = "0.29.0" }\n'
    if section == "dependencies":
        source = source.replace("[dependencies]\n", "[dependencies]\n" + addition)
    else:
        source += f"\n[{section}]\n{addition}"
    manifest.write_text(source)

    errors = check_contract(tmp_path)

    assert len(errors) == 1
    assert "unclassified" in errors[0]
    assert "pyo3" in errors[0]


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


def test_engine_dependencies_are_exact_and_consistent_across_locks() -> None:
    core = _core_dependencies("=0.12.0", "=0.10.1")
    locks = {
        Path("Cargo.lock"): _lock_packages("0.12.0", "0.10.1"),
        Path("adapter/Cargo.lock"): _lock_packages("0.12.0", "0.10.1"),
    }

    assert _engine_dependency_errors(core, locks) == []


def test_engine_dependencies_require_exact_manifest_versions() -> None:
    core = _core_dependencies("0.12.0", "=0.10.1")

    errors = _engine_dependency_errors(core, {})

    assert errors == ["portable core must pin biblatex to one exact release with =<version>"]


def test_engine_dependencies_reject_lock_drift_and_unapproved_sources() -> None:
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

    errors = _engine_dependency_errors(core, locks)

    assert errors == [
        "Cargo.lock must resolve biblatex 0.12.0",
        "Cargo.lock must resolve hayagriva from git+https://github.com/typst/hayagriva?rev=e7a9e7cecbbf774fd0d5226faeec12a7f8481a2e#e7a9e7cecbbf774fd0d5226faeec12a7f8481a2e",
        "adapter/Cargo.lock must resolve exactly one biblatex package",
        "adapter/Cargo.lock must resolve exactly one hayagriva package",
    ]


def _core_dependencies(biblatex: str, hayagriva: str) -> dict[str, Any]:
    return {
        "dependencies": {
            "biblatex": {"version": biblatex},
            "hayagriva": {
                "version": hayagriva,
                "git": "https://github.com/typst/hayagriva",
                "rev": "e7a9e7cecbbf774fd0d5226faeec12a7f8481a2e",
            },
        }
    }


def _lock_packages(biblatex: str, hayagriva: str) -> dict[str, Any]:
    source = "registry+https://github.com/rust-lang/crates.io-index"
    return {
        "package": [
            {"name": "biblatex", "version": biblatex, "source": source},
            {
                "name": "hayagriva",
                "version": hayagriva,
                "source": "git+https://github.com/typst/hayagriva?rev=e7a9e7cecbbf774fd0d5226faeec12a7f8481a2e#e7a9e7cecbbf774fd0d5226faeec12a7f8481a2e",
            },
        ]
    }


@pytest.mark.parametrize("manifest", ["Cargo.toml", "packages/polars-refkit/rust/Cargo.toml"])
def test_architecture_requires_audited_xml_patch_in_each_workspace(
    contract_root: Path, manifest: str
) -> None:
    path = contract_root / manifest
    path.write_text(path.read_text().replace("06a591e2f237d25e1dfdedac3f3d1494c496c52d", "0" * 40))

    errors = check_contract(contract_root)

    assert any(error.startswith(f"{manifest} must patch citationberg") for error in errors)


@pytest.mark.parametrize("lockfile", ["Cargo.lock", "packages/polars-refkit/rust/Cargo.lock"])
def test_architecture_requires_safe_xml_resolution_in_each_lock(
    contract_root: Path, lockfile: str
) -> None:
    path = contract_root / lockfile
    path.write_text(
        path.read_text().replace(
            'name = "quick-xml"\nversion = "0.41.0"', 'name = "quick-xml"\nversion = "0.38.4"'
        )
    )

    assert f"{lockfile} must resolve quick-xml >=0.41.0 from crates.io" in check_contract(
        contract_root
    )


@pytest.mark.parametrize("change", ["source", "version", "duplicate"])
def test_architecture_requires_one_audited_citationberg(contract_root: Path, change: str) -> None:
    path = contract_root / "Cargo.lock"
    source = path.read_text()
    if change == "source":
        source = source.replace(
            "git+https://github.com/typst/citationberg?rev=06a591e2f237d25e1dfdedac3f3d1494c496c52d#06a591e2f237d25e1dfdedac3f3d1494c496c52d",
            "registry+https://github.com/rust-lang/crates.io-index",
        )
    elif change == "version":
        source = source.replace(
            'name = "citationberg"\nversion = "0.7.0"', 'name = "citationberg"\nversion = "0.8.0"'
        )
    else:
        source += '\n[[package]]\nname = "citationberg"\nversion = "0.7.0"\nsource = "registry+https://github.com/rust-lang/crates.io-index"\n'
    path.write_text(source)

    assert any(
        "Cargo.lock must resolve one citationberg" in error
        for error in check_contract(contract_root)
    )


def test_architecture_requires_the_core_xml_parser_pin(contract_root: Path) -> None:
    path = contract_root / "crates/refkit-core/Cargo.toml"
    path.write_text(path.read_text().replace('quick-xml = "=0.41.0"', 'quick-xml = "0.41.0"'))

    assert "portable core must pin quick-xml =0.41.0 from crates.io" in check_contract(
        contract_root
    )
