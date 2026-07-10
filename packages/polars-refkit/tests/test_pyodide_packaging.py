from __future__ import annotations

import importlib.util
import tomllib
from pathlib import Path
from typing import Any, cast

import pytest
import yaml

ROOT = Path(__file__).resolve().parents[3]


def read_text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def read_toml(path: str) -> dict[str, Any]:
    return tomllib.loads(read_text(path))


def read_workflow_jobs(path: str) -> dict[str, Any]:
    workflow = cast(dict[str, Any], yaml.safe_load(read_text(path)))
    return cast(dict[str, Any], workflow["jobs"])


def load_module(path: str) -> Any:
    module_path = ROOT / path
    spec = importlib.util.spec_from_file_location(module_path.stem, module_path)
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_public_package_keeps_polars_dependency_floor() -> None:
    data = read_toml("packages/polars-refkit/pyproject.toml")
    project = cast(dict[str, Any], data["project"])
    build_system = cast(dict[str, Any], data["build-system"])

    assert project["dependencies"] == ["polars>=1.29"]
    assert "polars>=1.29" in build_system["requires"]


def test_workflows_build_and_test_polars_refkit_pyemscripten_wheels() -> None:
    ci_jobs = read_workflow_jobs(".github/workflows/ci.yml")
    publish_jobs = read_workflow_jobs(".github/workflows/publish.yml")
    release_jobs = read_workflow_jobs(".github/workflows/release-tests-polars-refkit.yml")

    assert_polars_wasm_build_job(
        ci_jobs["build-polars-refkit-wasm"],
        artifact_name="polars_refkit_pypi_files_pyemscripten_${{ matrix.python-version }}",
    )
    assert_polars_wasm_build_job(
        publish_jobs["build-polars-refkit-wasm"],
        artifact_name="release_polars_refkit_pyemscripten_${{ matrix.python-version }}",
    )
    assert_polars_pyemscripten_smoke_job(
        ci_jobs["test-polars-refkit-pyemscripten"],
        artifact_names=[
            "refkit_core_pypi_files_pyemscripten_${{ matrix.python-version }}",
            "refkit_pypi_files",
            "polars_refkit_pypi_files_pyemscripten_${{ matrix.python-version }}",
        ],
    )
    assert_polars_pyemscripten_smoke_job(
        release_jobs["test-pyemscripten"],
        artifact_names=[
            "release_refkit_dist",
            "release_refkit_core_pyemscripten_3.14",
            "release_polars_refkit_pyemscripten_3.14",
        ],
    )

    assert {name for name in publish_jobs if name.startswith("publish-")} == {
        "publish-refkit-core",
        "publish-refkit",
        "publish-polars-refkit",
    }
    assert publish_jobs["test-polars-refkit"]["uses"] == (
        "./.github/workflows/release-tests-polars-refkit.yml"
    )
    assert "with" not in publish_jobs["test-polars-refkit"]


def assert_polars_wasm_build_job(job: dict[str, Any], *, artifact_name: str) -> None:
    steps = cast(list[dict[str, Any]], job["steps"])
    setup_step = next(
        step for step in steps if step.get("uses") == "./.github/actions/setup-pyodide"
    )
    build_step = next(
        step for step in steps if str(step.get("uses", "")).startswith("PyO3/maturin-action")
    )
    build_env = cast(dict[str, Any], build_step["env"])
    build_with = cast(dict[str, Any], build_step["with"])
    upload_names = [
        cast(dict[str, Any], step["with"])["name"]
        for step in steps
        if str(step.get("uses", "")).startswith("actions/upload-artifact")
    ]

    assert setup_step["id"] == "pyodide"
    assert build_with["working-directory"] == "packages/polars-refkit"
    assert build_with["target"] == "wasm32-unknown-emscripten"
    assert build_with["rust-toolchain"] == "${{ steps.pyodide.outputs.rust-toolchain }}"
    assert build_env == {
        "CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS": (
            "${{ steps.pyodide.outputs.rustflags }}"
        ),
        "HOST_CC": "cc",
        "MATURIN_PYEMSCRIPTEN_PLATFORM_VERSION": (
            "${{ steps.pyodide.outputs.pyodide-abi-version }}"
        ),
    }
    assert upload_names == [artifact_name]


def assert_polars_pyemscripten_smoke_job(
    job: dict[str, Any],
    *,
    artifact_names: list[str],
) -> None:
    steps = cast(list[dict[str, Any]], job["steps"])
    download_names = [
        cast(dict[str, Any], step["with"])["name"]
        for step in steps
        if str(step.get("uses", "")).startswith("actions/download-artifact")
    ]
    run_steps = "\n".join(str(step.get("run", "")) for step in steps)

    assert download_names == artifact_names
    assert "python .github/pyodide/smoke_polars_refkit.py" in run_steps


def test_benchmark_package_stays_outside_public_runtime_dependencies() -> None:
    workspace = read_toml("pyproject.toml")
    bench = read_toml("packages/refkit-bench/pyproject.toml")

    public_project = cast(dict[str, Any], workspace["project"])
    bench_dependencies = set(cast(list[str], bench["project"]["dependencies"]))
    bench_sources = cast(dict[str, Any], bench["tool"]["uv"]["sources"])
    root_sources = cast(dict[str, Any], workspace["tool"]["uv"]["sources"])

    assert set(public_project["dependencies"]) == {"refkit-core", "refkit", "polars-refkit"}
    assert {"refkit", "polars-refkit"} <= bench_dependencies
    assert bench_sources["refkit"] == {"workspace": True}
    assert bench_sources["polars-refkit"] == {"workspace": True}
    assert root_sources["refkit"] == {"workspace": True}
    assert root_sources["polars-refkit"] == {"workspace": True}


def test_pyodide_smoke_runs_through_public_polars_expressions(
    capsys: pytest.CaptureFixture[str],
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    module = load_module(".github/pyodide/smoke_polars_refkit.py")

    module.main()

    lines = capsys.readouterr().out.splitlines()
    assert lines[-1] == "(Doe, 2024)"
    assert any(line.startswith("polars ") for line in lines)
    assert any(line.startswith("polars-refkit ") for line in lines)

    monkeypatch.setattr(module.rk, "check_refkit_core_version", lambda: False)
    with pytest.raises(AssertionError):
        module.main()
