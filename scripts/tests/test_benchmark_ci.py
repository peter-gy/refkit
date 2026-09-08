from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import pytest

from scripts import benchmark_ci, benchmark_report

BASE = "a" * 40
CANDIDATE = "b" * 40


def comparison(ratio=0.8, interval=(0.75, 0.85)):
    return {
        "case": "parse.bibtex/real/refkit",
        "baseline_seconds": 0.002,
        "candidate_seconds": 0.002 * ratio,
        "candidate_over_baseline": ratio,
        "ratio_interval_95": list(interval),
        "baseline_workers": 5,
        "candidate_workers": 5,
    }


def report():
    return {
        "schema": 1,
        "threshold": 0.05,
        "baseline_sha": BASE,
        "candidate_sha": CANDIDATE,
        "platforms": {
            name: {"status": "complete", "comparison": {"rows": [comparison()]}}
            for name in ("Linux", "Windows", "macOS")
        },
    }


@pytest.mark.parametrize(
    "ratio,interval,state",
    [
        (0.8, (0.75, 0.85), "faster"),
        (1.2, (1.15, 1.25), "slower"),
        (1.01, (0.99, 1.02), "same"),
        (1.2, (0.98, 1.3), "inconclusive"),
    ],
)
def test_markdown_classifies_effects_with_uncertainty(ratio, interval, state):
    data = report()
    data["platforms"]["Linux"]["comparison"]["rows"] = [comparison(ratio, interval)]
    text = benchmark_report.markdown(data)
    assert f"| {state} |" in text
    assert f"{(ratio - 1) * 100:+.1f}%" in text
    assert "95% change interval" in text


@pytest.mark.parametrize(
    "field,value",
    [
        ("case", "@everyone <script>"),
        ("candidate_seconds", float("nan")),
        ("ratio_interval_95", [1.2, 0.8]),
        ("baseline_workers", 4),
        ("candidate_over_baseline", 2),
    ],
)
def test_publisher_rejects_invalid_measurement_data(field, value):
    data = report()
    data["platforms"]["Linux"]["comparison"]["rows"][0][field] = value
    with pytest.raises(ValueError):
        benchmark_report.markdown(data)


def test_cli_consolidates_all_platforms_and_writes_job_summary(tmp_path):
    for platform in ("Linux", "Windows", "macOS"):
        directory = tmp_path / platform
        directory.mkdir()
        data = report()["platforms"][platform]
        data.update(baseline_sha=BASE, candidate_sha=CANDIDATE)
        (directory / "comparison.json").write_text(json.dumps(data))
    import os

    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "scripts.benchmark_report",
            str(tmp_path),
            "--baseline",
            BASE,
            "--candidate",
            CANDIDATE,
        ],
        env={**os.environ, "GITHUB_STEP_SUMMARY": str(tmp_path / "job.md")},
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    consolidated = json.loads((tmp_path / "report.json").read_text())
    assert set(consolidated["platforms"]) == {"Linux", "Windows", "macOS"}
    assert (tmp_path / "summary.md").read_bytes() == (tmp_path / "job.md").read_bytes()


def test_missing_platform_produces_failed_summary_and_exit_status(tmp_path, monkeypatch):
    monkeypatch.setattr(
        sys,
        "argv",
        ["benchmark-report", str(tmp_path), "--baseline", BASE, "--candidate", CANDIDATE],
    )
    assert benchmark_report.main() == 1
    assert "Measurement failed" in (tmp_path / "summary.md").read_text()


def test_runner_builds_two_revisions_and_compares_one_shared_harness(tmp_path, monkeypatch):
    commands = []

    def execute(command, *, env, capture=False):
        commands.append((command, env))
        if command[0] == "git":
            return BASE if command[2].endswith("base") else CANDIDATE
        if "compare" in command:
            return json.dumps({"rows": [comparison()]})
        return ""

    monkeypatch.setattr(benchmark_ci, "execute", execute)
    monkeypatch.setenv("GITHUB_RUN_NUMBER", "2")
    monkeypatch.setenv("UV_PYTHON", "3.14")
    output = tmp_path / "results"
    assert benchmark_ci.run(tmp_path / "head", tmp_path / "base", output, "Linux") == 0
    data = json.loads((output / "comparison.json").read_text())
    assert data["execution_order"] == ["baseline", "candidate"]
    assert (data["baseline_sha"], data["candidate_sha"]) == (BASE, CANDIDATE)
    syncs = [cmd for cmd, _ in commands if cmd[:2] == ["uv", "sync"]]
    assert all(cmd[cmd.index("--project") + 1] == str(tmp_path / "head") for cmd in syncs)
    assert all(cmd[cmd.index("--python") + 1] == "3.14" for cmd in syncs)
    installs = [cmd for cmd, _ in commands if cmd[:3] == ["uv", "pip", "install"]]
    assert installs[0][-2:] == [
        str(tmp_path / "base/packages/refkit"),
        str(tmp_path / "base/packages/polars-refkit"),
    ]
    assert installs[1][-2:] == [
        str(tmp_path / "head/packages/refkit"),
        str(tmp_path / "head/packages/polars-refkit"),
    ]
    assert all(
        env["MATURIN_PEP517_ARGS"] == "--profile release --locked"
        for cmd, env in commands
        if cmd[0] == "uv"
    )


def test_failed_build_retains_revision_provenance(tmp_path, monkeypatch):
    def execute(command, **kwargs):
        if command[0] == "git":
            return BASE
        raise subprocess.CalledProcessError(1, command)

    monkeypatch.setattr(benchmark_ci, "execute", execute)
    output = tmp_path / "results"
    assert benchmark_ci.run(tmp_path, tmp_path, output, "Windows") == 1
    data = json.loads((output / "comparison.json").read_text())
    assert data["status"] == "failed"
    assert data["baseline_sha"] == BASE


def test_runner_installs_each_compiled_revision_with_inherited_build_directories(
    tmp_path, monkeypatch
):
    cargo = shutil.which("cargo") or ""
    if not cargo:
        pytest.skip("Cargo is required for the native build isolation check")
    roots = {name: tmp_path / name for name in ("baseline", "candidate")}
    for name, root in roots.items():
        (root / "src").mkdir(parents=True)
        (root / "Cargo.toml").write_text(
            '[package]\nname = "revision_probe"\nversion = "0.1.0"\nedition = "2021"\n[workspace]\n'
        )
        source = root / "src/main.rs"
        source.write_text(f'fn main() {{ println!("{name}"); }}\n')
        # CI checks out both sources before either build writes its dependency timestamps.
        os.utime(source, (1_600_000_000, 1_600_000_000))
    shared = tmp_path / "shared-target"
    monkeypatch.setenv("CARGO_TARGET_DIR", str(shared))
    monkeypatch.setenv("CARGO_BUILD_BUILD_DIR", str(shared))
    monkeypatch.setenv("GITHUB_RUN_NUMBER", "2")
    binary_name = "revision_probe.exe" if os.name == "nt" else "revision_probe"
    observed = []

    def execute(command, *, env, capture=False):
        if command[0] == "git":
            return BASE if Path(command[2]).name == "baseline" else CANDIDATE
        if command[:3] == ["uv", "pip", "install"]:
            root = Path(command[-2]).parents[1]
            subprocess.run(
                [
                    cargo,
                    "build",
                    "--offline",
                    "--release",
                    "--manifest-path",
                    str(root / "Cargo.toml"),
                ],
                env=env,
                check=True,
                capture_output=True,
                timeout=60,
            )
            installed = Path(command[command.index("--python") + 1]).parent / binary_name
            installed.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(Path(env["CARGO_TARGET_DIR"]) / "release" / binary_name, installed)
        elif "run" in command:
            installed = Path(command[0]).parent / binary_name
            observed.append(
                subprocess.check_output([str(installed)], text=True, timeout=10).strip()
            )
        elif "compare" in command:
            return json.dumps({"rows": [comparison()]})
        return ""

    monkeypatch.setattr(benchmark_ci, "execute", execute)
    assert (
        benchmark_ci.run(roots["candidate"], roots["baseline"], tmp_path / "results", "Linux") == 0
    )
    assert observed == ["baseline", "candidate"]
