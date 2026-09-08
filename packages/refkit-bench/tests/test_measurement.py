from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

import pytest

from refkit_bench import model, runner, worker
from refkit_bench.cases import Case
from refkit_bench.model import Prepared
from refkit_bench.process import execute
from refkit_bench.report import load_result


def test_batch_clock_excludes_validation(monkeypatch: pytest.MonkeyPatch):
    clock = [0.0]
    received = []

    def operation():
        clock[0] += 1
        return clock[0]

    def check(result):
        received.append(result)
        clock[0] += 100

    monkeypatch.setattr(model, "perf_counter", lambda: clock[0])
    prepared = Prepared(operation, check)
    assert prepared.time_batch(3) == 3.0
    assert received == [3.0]


def test_runtime_batch_clock_is_checked():
    prepared = Prepared(
        lambda: None, lambda value: None, measure=lambda loops: (float("nan"), None)
    )
    with pytest.raises(ValueError, match="finite positive"):
        prepared.time_batch(1)
    with pytest.raises(ValueError, match="at least one"):
        prepared.time_batch(0)


@pytest.mark.parametrize(
    "selection",
    [
        ["--lane", "unknown"],
        ["--dataset", "unknown"],
        ["--lane", "parse.bibtex", "--package", "bibtex-tidy"],
        ["--lane", "format.spec", "--dataset", "tiny"],
    ],
)
def test_cli_rejects_unknown_or_incompatible_cases(selection, capsys):
    with pytest.raises(SystemExit) as error:
        runner.main(["list", *selection])
    assert error.value.code == 2
    assert "error:" in capsys.readouterr().err


def test_validation_failure_keeps_artifact_identity_and_closes(monkeypatch: pytest.MonkeyPatch):
    closed = []

    def reject(value):
        raise AssertionError("title changed")

    prepared = Prepared(
        lambda: "wrong",
        reject,
        metadata={"input_sha256": "input"},
        close=lambda: closed.append(True),
    )
    monkeypatch.setattr(Case, "prepare", lambda self: prepared)
    monkeypatch.setattr(worker, "artifact", lambda package: {"artifact_sha256": "measured-library"})
    result = worker.inspect_case("parse.bibtex/real/refkit")
    assert result["status"] == "failed"
    assert result["artifact"]["artifact_sha256"] == "measured-library"
    assert result["contract"]["input_sha256"] == "input"
    assert "title changed" in result["detail"]
    assert closed == [True]


def test_cleanup_failure_invalidates_the_case(monkeypatch: pytest.MonkeyPatch):
    def close():
        raise RuntimeError("worker still active")

    monkeypatch.setattr(
        Case, "prepare", lambda self: Prepared(lambda: "ok", lambda value: None, close=close)
    )
    monkeypatch.setattr(worker, "artifact", lambda package: {})
    result = worker.inspect_case("parse.bibtex/real/refkit")
    assert result["status"] == "failed"
    assert "cleanup failed" in result["detail"]


def test_failed_preflight_writes_a_failure_manifest(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
):
    output = tmp_path / "failed"
    monkeypatch.setattr(
        runner,
        "check_cases",
        lambda *args: [
            {"name": "parse.bibtex/real/refkit", "status": "failed", "detail": "missing author"}
        ],
    )
    assert runner.main(["run", "--output", str(output), "--smoke"]) == 1
    manifest = json.loads((output / "manifest.json").read_text())
    assert manifest["status"] == "failed"
    assert manifest["checks"][0]["detail"] == "missing author"
    assert {path.name for path in output.iterdir()} == {"manifest.json"}


def test_worker_crash_is_reported_with_stderr(monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setattr(
        runner,
        "execute",
        lambda *args: subprocess.CompletedProcess([], 1, "", "native failure detail"),
    )
    row = runner.check_cases([Case("parse.bibtex", "tiny", "refkit")], 1)[0]
    assert row["status"] == "failed"
    assert "native failure detail" in row["detail"]


def test_process_output_and_timeout_are_bounded():
    result = execute([sys.executable, "-c", 'print("complete")'], 10)
    assert result.returncode == 0
    assert result.stdout == "complete\n"
    with pytest.raises(TimeoutError, match="exceeded"):
        execute([sys.executable, "-c", "import time; time.sleep(60)"], 0.05)


def test_cli_measures_native_and_node_through_real_workers(tmp_path: Path):
    output = tmp_path / "smoke"
    assert (
        runner.main(
            [
                "run",
                "--lane",
                "format.layout",
                "--dataset",
                "tiny",
                "--output",
                str(output),
                "--smoke",
                "--seed",
                "17",
            ]
        )
        == 0
    )
    manifest, suite = load_result(output)
    assert manifest["profile"] == "smoke"
    assert manifest["status"] == "complete"
    assert set(suite.get_benchmark_names()) == {
        "format.layout/tiny/refkit",
        "format.layout/tiny/bibtex-tidy",
    }
    assert all(benchmark.get_nvalue() == 1 for benchmark in suite)
    assert runner.main(["report", str(output), "--json"]) == 0
    with pytest.raises(SystemExit) as error:
        runner.main(["compare", str(output), str(output)])
    assert error.value.code == 2


def test_existing_result_directory_is_preserved(tmp_path: Path):
    (tmp_path / "baseline.txt").write_text("keep")
    with pytest.raises(SystemExit) as error:
        runner.main(["run", "--output", str(tmp_path), "--smoke"])
    assert error.value.code == 2
    assert (tmp_path / "baseline.txt").read_text() == "keep"


def test_measurement_requires_an_observed_release_build(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
):
    monkeypatch.setattr(
        runner,
        "check_cases",
        lambda *args: [
            {
                "name": "parse.bibtex/real/refkit",
                "status": "ok",
                "artifact": {"build_mode": "debug"},
            }
        ],
    )
    output = tmp_path / "debug"
    assert runner.main(["run", "--package", "refkit", "--output", str(output)]) == 1
    manifest = json.loads((output / "manifest.json").read_text())
    assert manifest["status"] == "failed"
    assert manifest["checks"][0]["status"] == "failed"
    assert "observed release build" in manifest["checks"][0]["detail"]


def test_partial_measurements_are_retained_as_failed_evidence(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
):
    import pyperf

    names = ["inspect.keys/tiny/refkit", "inspect.project/tiny/refkit"]
    checks = [
        {
            "name": name,
            "status": "ok",
            "artifact": {"build_mode": "release"},
            "case_sha256": name,
            "contract": {},
        }
        for name in names
    ]
    monkeypatch.setattr(runner, "check_cases", lambda *args: checks)
    monkeypatch.setattr(runner, "environment", lambda: {"benchmark_sha256": "source"})
    completed = []

    def measure(command, timeout):
        name = command[command.index("--case") + 1]
        output = Path(command[command.index("--output") + 1])
        if completed:
            output.write_text('{"partial":')
            return subprocess.CompletedProcess(command, 1, "", "worker lost its native library")
        run = pyperf.Run(
            [0.001],
            metadata={
                "name": name,
                "loops": 1,
                "build_mode": "release",
                "case_sha256": name,
                "benchmark_sha256": "source",
            },
            collect_metadata=False,
        )
        pyperf.BenchmarkSuite([pyperf.Benchmark([run])]).dump(str(output))
        completed.append(name)
        return subprocess.CompletedProcess(command, 0, "", "")

    monkeypatch.setattr(runner, "execute", measure)
    output = tmp_path / "partial"
    assert (
        runner.main(
            [
                "run",
                "--lane",
                "inspect.keys",
                "--lane",
                "inspect.project",
                "--dataset",
                "tiny",
                "--package",
                "refkit",
                "--output",
                str(output),
                "--smoke",
            ]
        )
        == 1
    )
    manifest = json.loads((output / "manifest.json").read_text())
    assert manifest["status"] == "failed"
    assert manifest["failed_case"] in set(names) - set(completed)
    assert "worker lost its native library" in manifest["detail"]
    suite = pyperf.BenchmarkSuite.load(str(output / "timings.json"))
    assert suite.get_benchmark_names() == completed
    with pytest.raises(ValueError, match="complete benchmark run"):
        load_result(output)


def test_cli_lists_and_checks_each_selected_case_once(capsys: pytest.CaptureFixture[str]):
    selection = [
        "--lane",
        "inspect.keys",
        "--lane",
        "inspect.keys",
        "--dataset",
        "tiny",
        "--dataset",
        "tiny",
        "--package",
        "refkit",
    ]
    assert runner.main(["list", *selection, "--json"]) == 0
    listed = json.loads(capsys.readouterr().out)
    assert [(row["case"], row["unit"]) for row in listed] == [
        ("inspect.keys/tiny/refkit", "library")
    ]
    assert runner.main(["check", *selection]) == 0
    checked = json.loads(capsys.readouterr().out)
    assert [(row["name"], row["status"]) for row in checked["checks"]] == [
        ("inspect.keys/tiny/refkit", "ok")
    ]
    assert checked["checks"][0]["contract"]["record_count"] == 3


@pytest.mark.skipif(sys.platform == "win32", reason="POSIX process-group lifecycle")
def test_interruption_terminates_the_worker_and_its_child(monkeypatch: pytest.MonkeyPatch):
    import os
    import signal

    communicate = subprocess.Popen.communicate
    interrupted = False
    child_pid = None
    worker_process = None

    def interrupt(process, *args, **kwargs):
        nonlocal interrupted, child_pid, worker_process
        if not interrupted:
            interrupted = True
            worker_process = process
            child_pid = int(process.stdout.readline())
            raise KeyboardInterrupt
        return communicate(process, *args, **kwargs)

    monkeypatch.setattr(subprocess.Popen, "communicate", interrupt)
    program = (
        "import subprocess,sys,signal; "
        "child=subprocess.Popen([sys.executable,'-u','-c',"
        "\"import signal; print('ready', flush=True); signal.pause()\"],"
        "stdout=subprocess.PIPE,text=True); "
        "child.stdout.readline(); print(child.pid,flush=True); signal.pause()"
    )
    try:
        with pytest.raises(KeyboardInterrupt):
            execute([sys.executable, "-u", "-c", program], 10)
        assert worker_process is not None and worker_process.poll() is not None
        assert child_pid is not None
        state = subprocess.run(
            ["ps", "-p", str(child_pid), "-o", "stat="], capture_output=True, text=True
        )
        assert not state.stdout.strip() or state.stdout.strip().startswith("Z")
    finally:
        if child_pid is not None:
            try:
                os.kill(child_pid, signal.SIGKILL)
            except ProcessLookupError:
                pass


def test_cli_check_retains_failures_and_continues_to_remaining_cases(
    capsys: pytest.CaptureFixture[str],
):
    assert (
        runner.main(
            [
                "check",
                "--lane",
                "format.spec",
                "--package",
                "bibtex-tidy",
                "--dataset",
                "numeric",
                "--dataset",
                "defaults",
                "--dataset",
                "generate-keys",
            ]
        )
        == 1
    )
    report = json.loads(capsys.readouterr().out)
    outcomes = {row["name"]: row for row in report["checks"]}
    assert {name: row["status"] for name, row in outcomes.items()} == {
        "format.spec/numeric/bibtex-tidy": "failed",
        "format.spec/defaults/bibtex-tidy": "ok",
        "format.spec/generate-keys/bibtex-tidy": "failed",
    }
    assert "exact formatter output" in outcomes["format.spec/numeric/bibtex-tidy"]["detail"]
    assert outcomes["format.spec/numeric/bibtex-tidy"]["artifact"]["package_version"] == "1.14.0"


@pytest.mark.parametrize("option,value", [("--processes", "0"), ("--min-time", "nan")])
def test_cli_rejects_invalid_sampling_budgets_before_creating_results(
    tmp_path: Path, option: str, value: str, capsys: pytest.CaptureFixture[str]
):
    output = tmp_path / "invalid"
    with pytest.raises(SystemExit) as error:
        runner.main(["run", "--output", str(output), option, value])
    assert error.value.code == 2
    assert "positive" in capsys.readouterr().err
    assert not output.exists()


def test_cli_validates_the_real_capability_matrix(tmp_path: Path, capsys):
    output = tmp_path / "checks.json"
    assert (
        runner.main(["check", "--lane", "all", "--dataset", "real", "--output", str(output)]) == 0
    )
    report = json.loads(output.read_text())
    rows = {row["name"]: row for row in report["checks"]}
    participants = {
        "parse.bibtex": {"refkit", "bibtexparser", "pybtex"},
        "inspect.keys": {"refkit", "bibtexparser", "pybtex"},
        "inspect.lookup": {"refkit", "bibtexparser", "pybtex"},
        "inspect.project": {"refkit", "bibtexparser", "pybtex"},
        "raw.edit": {"refkit", "bibtexparser"},
        "render.citation": {"refkit", "citeproc-py"},
        "render.bibliography": {"refkit", "citeproc-py"},
        "render.document": {"refkit", "citeproc-py"},
        "format.layout": {"refkit", "bibtex-tidy"},
        "format.keys": {"refkit", "bibtex-tidy"},
        "batch.parse": {"polars-eager", "polars-lazy"},
        "batch.cite": {"polars-eager", "polars-lazy"},
    }
    assert set(rows) == {
        f"{lane}/real/{package}" for lane, packages in participants.items() for package in packages
    }
    assert rows["batch.parse/real/polars-lazy"]["contract"]["row_count"] == 12
    assert rows["batch.parse/real/polars-lazy"]["contract"]["polars_threads"] == 1
    assert json.loads(capsys.readouterr().out)["schema"] == 1


def test_cli_validates_upstream_source_options_as_structured_data(capsys):
    assert (
        runner.main(
            ["check", "--lane", "format.spec", "--dataset", "numeric", "--package", "refkit"]
        )
        == 0
    )
    row = json.loads(capsys.readouterr().out)["checks"][0]
    assert row["status"] == "ok"
    assert row["contract"]["options"] == {"numeric": True}
    assert row["contract"]["source_repository"] == "https://github.com/FlamingTempura/bibtex-tidy"


def test_source_mutation_invalidates_completed_samples(tmp_path: Path, monkeypatch):
    monkeypatch.setattr(runner, "source_digest", lambda: "changed after measurement")
    output = tmp_path / "changed-source"
    assert (
        runner.main(
            [
                "run",
                "--lane",
                "inspect.keys",
                "--dataset",
                "tiny",
                "--package",
                "refkit",
                "--smoke",
                "--output",
                str(output),
            ]
        )
        == 1
    )
    manifest = json.loads((output / "manifest.json").read_text())
    assert manifest["status"] == "failed"
    assert manifest["detail"] == "benchmark sources changed during measurement"
    with pytest.raises(ValueError, match="complete benchmark run"):
        load_result(output)


def test_missing_worker_archive_is_a_recorded_failure(tmp_path: Path, monkeypatch):
    monkeypatch.setattr(
        runner,
        "check_cases",
        lambda *args: [
            {
                "name": "parse.bibtex/real/refkit",
                "status": "ok",
                "artifact": {"build_mode": "release"},
            }
        ],
    )
    monkeypatch.setattr(runner, "execute", lambda *args: subprocess.CompletedProcess([], 0, "", ""))
    output = tmp_path / "missing-archive"
    assert runner.main(["run", "--package", "refkit", "--smoke", "--output", str(output)]) == 1
    manifest = json.loads((output / "manifest.json").read_text())
    assert manifest["status"] == "failed"
    assert manifest["failed_case"] == "parse.bibtex/real/refkit"


def test_cli_interruption_retains_an_incomplete_manifest(tmp_path: Path, monkeypatch, capsys):
    def interrupt(*args):
        raise KeyboardInterrupt

    monkeypatch.setattr(runner, "check_cases", interrupt)
    output = tmp_path / "interrupted"
    assert runner.main(["run", "--output", str(output), "--smoke"]) == 130
    assert json.loads((output / "manifest.json").read_text())["status"] == "interrupted"
    assert "Benchmark interrupted" in capsys.readouterr().err


def test_workers_preserve_preflight_runtime_configuration(tmp_path: Path, monkeypatch):
    monkeypatch.setenv("PYTHONUTF8", "1")
    output = tmp_path / "configured-runtime"
    assert (
        runner.main(
            [
                "run",
                "--lane",
                "parse.bibtex",
                "--dataset",
                "tiny",
                "--package",
                "pybtex",
                "--smoke",
                "--output",
                str(output),
            ]
        )
        == 0
    )
    manifest, _ = load_result(output)
    flags = json.loads(manifest["checks"][0]["artifact"]["python_flags"])
    assert flags["utf8_mode"] == 1
