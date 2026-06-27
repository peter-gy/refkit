from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Any, cast

ROOT = Path(__file__).resolve().parents[3]


def read_text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def read_toml(path: str) -> dict[str, Any]:
    return tomllib.loads(read_text(path))


def test_public_package_keeps_polars_dependency_floor() -> None:
    data = read_toml("packages/polars-refkit/pyproject.toml")
    project = cast(dict[str, Any], data["project"])
    build_system = cast(dict[str, Any], data["build-system"])

    assert project["dependencies"] == ["polars>=1.29"]
    assert "polars>=1.29" in build_system["requires"]


def test_rust_plugin_uses_package_local_pyodide_abi_boundary() -> None:
    data = read_toml("packages/polars-refkit/rust/Cargo.toml")
    package = cast(dict[str, Any], data["package"])
    workspace = cast(dict[str, Any], data["workspace"])
    dependencies = cast(dict[str, Any], data["dependencies"])
    polars = cast(dict[str, Any], dependencies["polars"])
    polars_core = cast(dict[str, Any], dependencies["polars-core"])
    pyo3_polars = cast(dict[str, Any], dependencies["pyo3-polars"])

    assert package["version"] == "0.0.2"
    assert workspace["resolver"] == "3"
    assert polars["version"] == "=0.50.0"
    assert polars_core["version"] == "=0.50.0"
    assert dependencies["pyo3"] == "0.25"
    assert pyo3_polars["version"] == "=0.23.1"


def test_rust_plugin_exports_polars_plugin_version_callbacks() -> None:
    lib = read_text("packages/polars-refkit/rust/src/lib.rs")
    parse = read_text("packages/polars-refkit/rust/src/expressions/parse.rs")
    render = read_text("packages/polars-refkit/rust/src/expressions/render.rs")

    assert "_polars_plugin_get_version" in lib
    assert "_polars_plugin_get_last_error_message" in lib
    assert "#[polars_expr(output_type_func=uint32_output)]" in parse
    assert "#[polars_expr(output_type_func=boolean_output)]" in parse
    assert "#[polars_expr(output_type_func=string_output)]" in parse
    assert "#[polars_expr(output_type_func=string_output)]" in render


def test_workflows_build_and_test_polars_refkit_pyemscripten_wheels() -> None:
    ci = read_text(".github/workflows/ci.yml")
    publish = read_text(".github/workflows/publish.yml")
    release_tests = read_text(".github/workflows/release-tests.yml")

    for workflow in (ci, publish):
        assert "working-directory: packages/polars-refkit" in workflow
        assert "target: wasm32-unknown-emscripten" in workflow
        assert "CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS" in workflow
        assert "MATURIN_PYEMSCRIPTEN_PLATFORM_VERSION" in workflow
        assert "HOST_CC: cc" in workflow
        assert "rust_toolchain" in workflow
        assert "emscripten_version" in workflow
        assert "pyodide_abi_version" in workflow
        assert "rustflags" in workflow

    assert "polars_refkit_pypi_files_pyemscripten_${{ matrix.python-version }}" in ci
    assert "release_polars_refkit_pyemscripten_${{ matrix.python-version }}" in publish
    assert '"polars-refkit-rust": Path("packages/polars-refkit/rust/Cargo.toml")' in publish
    assert "release_polars_refkit_pyemscripten_3.14" in release_tests
    assert "smoke_polars_refkit.py" in ci
    assert "smoke_polars_refkit.py" in release_tests
    assert "refkit-bench" not in publish


def test_public_build_surface_excludes_refkit_bench() -> None:
    makefile = read_text("Makefile")
    workspace = read_toml("pyproject.toml")
    bench = read_toml("packages/refkit-bench/pyproject.toml")

    public_version = cast(dict[str, Any], workspace["project"])["version"]
    bench_project = cast(dict[str, Any], bench["project"])
    bench_dependencies = cast(list[str], bench_project["dependencies"])
    bench_sources = cast(dict[str, Any], bench["tool"]["uv"]["sources"])
    root_sources = cast(dict[str, Any], workspace["tool"]["uv"]["sources"])

    assert "uv build --all-packages" not in makefile
    assert "uv build --package refkit-core" in makefile
    assert "uv build --package refkit" in makefile
    assert "uv build --package polars-refkit" in makefile
    assert bench_project["version"] != public_version
    assert "refkit" in bench_dependencies
    assert "polars-refkit" in bench_dependencies
    assert not any(
        dependency.startswith(("refkit==", "polars-refkit==")) for dependency in bench_dependencies
    )
    assert bench_sources["refkit"] == {"workspace": True}
    assert bench_sources["polars-refkit"] == {"workspace": True}
    assert root_sources["refkit"] == {"workspace": True}
    assert root_sources["polars-refkit"] == {"workspace": True}


def test_pyodide_smoke_runs_through_public_polars_expressions() -> None:
    script = read_text(".github/pyodide/smoke_polars_refkit.py")

    assert "import polars as pl" in script
    assert "import polars_refkit" in script
    assert "import refkit as rk" in script
    assert "rk.check_refkit_core_version()" in script
    assert "namespace.entry_count()" in script
    assert 'namespace.cite("key", style="apa")' in script
    assert 'namespace.cite_rendered("key", style="apa")' in script
    assert 'namespace.cite_each("keys", style="apa")' in script
