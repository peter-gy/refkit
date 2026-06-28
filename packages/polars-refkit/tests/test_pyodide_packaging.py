from __future__ import annotations

import ast
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


def python_heredoc(run: str) -> str:
    lines = run.splitlines()
    start = lines.index("python - <<'PY'") + 1
    end = lines.index("PY", start)
    return "\n".join(lines[start:end])


def path_dict_assignment(source: str, name: str) -> dict[str, str]:
    module = ast.parse(source)
    for node in module.body:
        if not isinstance(node, ast.Assign):
            continue
        if not any(isinstance(target, ast.Name) and target.id == name for target in node.targets):
            continue
        if not isinstance(node.value, ast.Dict):
            break
        values: dict[str, str] = {}
        for key, value in zip(node.value.keys, node.value.values, strict=True):
            if (
                isinstance(key, ast.Constant)
                and isinstance(key.value, str)
                and isinstance(value, ast.Call)
                and isinstance(value.func, ast.Name)
                and value.func.id == "Path"
                and len(value.args) == 1
                and isinstance(value.args[0], ast.Constant)
                and isinstance(value.args[0].value, str)
            ):
                values[key.value] = value.args[0].value
        return values
    raise AssertionError(f"{name} assignment not found")


def make_target_commands(makefile: str, target: str) -> list[str]:
    lines = makefile.splitlines()
    target_line = f"{target}:"
    start = next(index for index, line in enumerate(lines) if line.startswith(target_line))
    commands: list[str] = []
    for line in lines[start + 1 :]:
        if line and not line.startswith(("\t", " ")):
            break
        if line.startswith("\t"):
            commands.append(line.strip())
    return commands


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


def test_rust_plugin_uses_package_local_pyodide_abi_boundary() -> None:
    data = read_toml("packages/polars-refkit/rust/Cargo.toml")
    dependencies = cast(dict[str, Any], data["dependencies"])
    polars = cast(dict[str, Any], dependencies["polars"])
    polars_core = cast(dict[str, Any], dependencies["polars-core"])
    pyo3_polars = cast(dict[str, Any], dependencies["pyo3-polars"])

    assert polars["version"] == "=0.50.0"
    assert polars_core["version"] == "=0.50.0"
    assert dependencies["pyo3"] == "0.25"
    assert pyo3_polars["version"] == "=0.23.1"


def test_workflows_build_and_test_polars_refkit_pyemscripten_wheels() -> None:
    ci_jobs = read_workflow_jobs(".github/workflows/ci.yml")
    publish_jobs = read_workflow_jobs(".github/workflows/publish.yml")
    release_jobs = read_workflow_jobs(".github/workflows/release-tests.yml")

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
        release_jobs["test-polars-refkit-pyemscripten"],
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
    assert publish_jobs["test-polars-refkit"]["uses"] == "./.github/workflows/release-tests.yml"
    assert publish_jobs["test-polars-refkit"]["with"] == {"package": "polars-refkit"}


def test_publish_release_version_gate_covers_public_manifests() -> None:
    publish_jobs = read_workflow_jobs(".github/workflows/publish.yml")
    version_step = next(
        step
        for step in cast(list[dict[str, Any]], publish_jobs["check-release-version"]["steps"])
        if step.get("id") == "version"
    )

    assert path_dict_assignment(python_heredoc(str(version_step["run"])), "version_files") == {
        "refkit": "packages/refkit/pyproject.toml",
        "refkit-core": "packages/refkit-core/pyproject.toml",
        "polars-refkit": "packages/polars-refkit/pyproject.toml",
        "polars-refkit-rust": "packages/polars-refkit/rust/Cargo.toml",
        "rust-workspace": "Cargo.toml",
    }


def assert_polars_wasm_build_job(job: dict[str, Any], *, artifact_name: str) -> None:
    steps = cast(list[dict[str, Any]], job["steps"])
    config_step = next(step for step in steps if step.get("id") == "pyodide-config")
    setup_step = next(
        step
        for step in steps
        if str(step.get("uses", "")).startswith("emscripten-core/setup-emsdk")
    )
    build_step = next(
        step for step in steps if str(step.get("uses", "")).startswith("PyO3/maturin-action")
    )
    config_run = str(config_step["run"])
    setup_with = cast(dict[str, Any], setup_step["with"])
    build_env = cast(dict[str, Any], build_step["env"])
    build_with = cast(dict[str, Any], build_step["with"])
    upload_names = [
        cast(dict[str, Any], step["with"])["name"]
        for step in steps
        if str(step.get("uses", "")).startswith("actions/upload-artifact")
    ]

    for config_name in ("rust_toolchain", "emscripten_version", "pyodide_abi_version", "rustflags"):
        assert config_name in config_run
    assert setup_with["version"] == "${{ steps.pyodide-config.outputs.emscripten-version }}"
    assert build_with["working-directory"] == "packages/polars-refkit"
    assert build_with["target"] == "wasm32-unknown-emscripten"
    assert build_with["rust-toolchain"] == "${{ steps.pyodide-config.outputs.rust-toolchain }}"
    assert build_env == {
        "CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS": (
            "${{ steps.pyodide-config.outputs.rustflags }}"
        ),
        "HOST_CC": "cc",
        "MATURIN_PYEMSCRIPTEN_PLATFORM_VERSION": (
            "${{ steps.pyodide-config.outputs.pyodide-abi-version }}"
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


def test_public_build_surface_excludes_refkit_bench() -> None:
    makefile = read_text("Makefile")
    workspace = read_toml("pyproject.toml")
    bench = read_toml("packages/refkit-bench/pyproject.toml")

    public_project = cast(dict[str, Any], workspace["project"])
    bench_dependencies = set(cast(list[str], bench["project"]["dependencies"]))
    bench_sources = cast(dict[str, Any], bench["tool"]["uv"]["sources"])
    root_sources = cast(dict[str, Any], workspace["tool"]["uv"]["sources"])

    assert set(public_project["dependencies"]) == {"refkit-core", "refkit", "polars-refkit"}
    assert make_target_commands(makefile, "build") == [
        "uv build --package refkit-core --no-create-gitignore",
        "uv build --package refkit --no-create-gitignore",
        "uv build --package polars-refkit --no-create-gitignore",
    ]
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
