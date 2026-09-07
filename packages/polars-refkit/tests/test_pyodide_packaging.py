from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Any, cast

import pytest
import yaml

from refkit_tests import smoke_polars_refkit

ROOT = Path(__file__).resolve().parents[3]


def read_text(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def read_toml(path: str) -> dict[str, Any]:
    return tomllib.loads(read_text(path))


def read_workflow_jobs(path: str) -> dict[str, Any]:
    workflow = cast(dict[str, Any], yaml.safe_load(read_text(path)))
    return cast(dict[str, Any], workflow["jobs"])


def test_public_package_keeps_polars_dependency_floor() -> None:
    data = read_toml("packages/polars-refkit/pyproject.toml")
    project = cast(dict[str, Any], data["project"])
    build_system = cast(dict[str, Any], data["build-system"])

    assert "polars>=1.29" in project["dependencies"]
    assert "polars>=1.29" in build_system["requires"]


def test_pyemscripten_runtime_consumes_the_built_wheel() -> None:
    jobs = read_workflow_jobs(".github/workflows/artifacts-polars-refkit.yml")
    build_id, build = next(
        (name, job)
        for name, job in jobs.items()
        if any(
            step.get("with", {}).get("target") == "wasm32-unknown-emscripten"
            for step in job.get("steps", [])
        )
    )
    runtime = next(
        job
        for job in jobs.values()
        if any(
            step.get("uses") == "./.github/actions/create-pyodide-venv"
            for step in job.get("steps", [])
        )
    )
    uploaded = next(
        step["with"]["name"]
        for step in build["steps"]
        if str(step.get("uses", "")).startswith("actions/upload-artifact@")
    )
    downloaded = next(
        step["with"]["name"]
        for step in runtime["steps"]
        if str(step.get("uses", "")).startswith("actions/download-artifact@")
    )
    versions = [row["python-version"] for row in build["strategy"]["matrix"]["include"]]
    assert downloaded in [
        uploaded.replace("${{ matrix.python-version }}", version) for version in versions
    ]
    dependencies = runtime["needs"]
    assert build_id in ([dependencies] if isinstance(dependencies, str) else dependencies)
    commands = "\n".join(step.get("run", "") for step in runtime["steps"])
    assert '"$workspace"/dist/polars_refkit-*.whl' in commands
    assert "--no-deps --no-index" in commands
    assert "python -m refkit_tests.smoke_polars_refkit" in commands


def test_benchmark_package_stays_outside_public_runtime_dependencies() -> None:
    workspace = read_toml("pyproject.toml")
    bench = read_toml("packages/refkit-bench/pyproject.toml")

    public_project = cast(dict[str, Any], workspace["project"])
    bench_dependencies = set(cast(list[str], bench["project"]["dependencies"]))
    bench_sources = cast(dict[str, Any], bench["tool"]["uv"]["sources"])
    root_sources = cast(dict[str, Any], workspace["tool"]["uv"]["sources"])

    assert set(public_project["dependencies"]) == {"refkit", "polars-refkit"}
    assert {"refkit", "polars-refkit"} <= bench_dependencies
    assert bench_sources["refkit"] == {"workspace": True}
    assert bench_sources["polars-refkit"] == {"workspace": True}
    assert root_sources["refkit"] == {"workspace": True}
    assert root_sources["polars-refkit"] == {"workspace": True}


def test_installed_smoke_runs_through_public_polars_expressions(
    capsys: pytest.CaptureFixture[str],
) -> None:
    smoke_polars_refkit.main()

    lines = capsys.readouterr().out.splitlines()
    assert lines[-1] == "(Doe, 2024)"
    assert any(line.startswith("polars ") for line in lines)
    assert any(line.startswith("polars-refkit ") for line in lines)
