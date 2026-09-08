from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from types import SimpleNamespace

import psutil
import pytest

from refkit_bench import affinity, runner
from refkit_bench.process import execute
from refkit_bench.report import load_result


@dataclass
class Process:
    mask: list[int]
    children_: list[Process] = field(default_factory=list)
    thread_ids: tuple[int, ...] = ()

    def cpu_affinity(self, cpus: list[int] | None = None) -> list[int]:
        if cpus is not None:
            self.mask = cpus
        return self.mask

    def children(self, *, recursive: bool) -> list[Process]:
        return self.children_

    def threads(self) -> list[SimpleNamespace]:
        return [SimpleNamespace(id=identifier) for identifier in self.thread_ids]


def test_cpu_selection_combines_indexes_ranges_and_duplicates():
    assert affinity.cpu_list("3, 1-2,2", 4) == (1, 2, 3)


@pytest.mark.parametrize("selection", ["0,4", "2-1", "-1", "all"])
def test_cpu_selection_rejects_invalid_bounds_or_syntax(selection: str):
    with pytest.raises(ValueError, match="CPU affinity"):
        affinity.cpu_list(selection, 4)


@pytest.mark.parametrize("selection,expected", [(None, (1, 2)), ("2-3", (2, 3))])
def test_configured_affinity_is_read_back_from_the_operating_system(
    monkeypatch: pytest.MonkeyPatch, selection: str | None, expected: tuple[int, ...]
):
    process = Process([1, 2])
    monkeypatch.setattr(affinity.psutil, "Process", lambda: process)
    monkeypatch.setattr(affinity.psutil, "cpu_count", lambda: 4)
    assert affinity.configure(selection) == expected
    assert process.mask == list(expected)


def test_configuration_rejects_an_unapplied_cpu_mask(monkeypatch: pytest.MonkeyPatch):
    process = SimpleNamespace(cpu_affinity=lambda cpus=None: [0, 1])
    monkeypatch.setattr(affinity.psutil, "Process", lambda: process)
    monkeypatch.setattr(affinity.psutil, "cpu_count", lambda: 2)
    with pytest.raises(ValueError, match="did not apply"):
        affinity.configure("1")


def test_explicit_affinity_requires_operating_system_support(monkeypatch: pytest.MonkeyPatch):
    monkeypatch.setattr(affinity.psutil, "Process", SimpleNamespace)
    assert affinity.configure(None) is None
    with pytest.raises(ValueError, match="unavailable"):
        affinity.configure("0")


@pytest.mark.parametrize("escape", ["owner", "child", "thread"])
def test_cpu_escape_is_rejected_for_the_complete_execution_tree(
    monkeypatch: pytest.MonkeyPatch, escape: str
):
    child = Process([1], thread_ids=(22,))
    owner = Process([1], children_=[child], thread_ids=(11,))
    monkeypatch.setattr(affinity.psutil, "Process", lambda: owner)
    monkeypatch.setattr(affinity.os, "sched_getaffinity", lambda tid: {1}, raising=False)
    affinity.verify((1,))
    if escape == "owner":
        owner.mask = [1, 2]
    elif escape == "child":
        child.mask = [2]
    else:
        monkeypatch.setattr(affinity.os, "sched_getaffinity", lambda tid: {2} if tid == 22 else {1})
    with pytest.raises(RuntimeError, match="escaped its declared CPU affinity"):
        affinity.verify((1,))


def test_exited_processes_and_threads_do_not_invalidate_live_affinity(
    monkeypatch: pytest.MonkeyPatch,
):
    def exited():
        raise psutil.NoSuchProcess(30)

    owner = Process([1], thread_ids=(11,))
    child = Process([1])
    monkeypatch.setattr(child, "cpu_affinity", exited)
    owner.children_.append(child)
    monkeypatch.setattr(affinity.psutil, "Process", lambda: owner)

    def exited_thread(tid):
        raise ProcessLookupError

    monkeypatch.setattr(affinity.os, "sched_getaffinity", exited_thread, raising=False)
    affinity.verify((1,))


def _current_cpus() -> list[int]:
    process = psutil.Process()
    assert hasattr(process, "cpu_affinity")
    return process.cpu_affinity()


@pytest.mark.skipif(sys.platform != "linux", reason="Linux process and thread affinity")
def test_node_process_and_threads_inherit_affinity_before_startup():
    selected = _current_cpus()[0]
    script = """
import json
import os
import sys
import psutil
from refkit_bench import affinity
from refkit_bench.formatting import _NodeWorker

cpus = affinity.configure(sys.argv[1])
worker = _NodeWorker()
try:
    worker.request({"input": "@article{a,title={A}}", "options": {}})
    worker.request({"loops": 1})
    process = psutil.Process(worker.process.pid)
    process_mask = process.cpu_affinity()
    thread_masks = [sorted(os.sched_getaffinity(thread.id)) for thread in process.threads()]
    affinity.verify(cpus)
    print(json.dumps({"process": process_mask, "threads": thread_masks}))
finally:
    worker.close()
"""
    completed = execute([sys.executable, "-c", script, str(selected)], 30)
    assert completed.returncode == 0, completed.stderr
    observed = json.loads(completed.stdout)
    assert observed["process"] == [selected]
    assert observed["threads"]
    assert all(mask == [selected] for mask in observed["threads"])


@pytest.mark.skipif(sys.platform != "linux", reason="Linux process and thread affinity")
def test_node_cli_smoke_records_effective_affinity_without_pinning_the_caller(tmp_path: Path):
    inherited = _current_cpus()
    selected = inherited[0]
    output = tmp_path / "node-affinity"
    assert (
        runner.main(
            [
                "run",
                "--lane",
                "format.layout",
                "--dataset",
                "tiny",
                "--package",
                "bibtex-tidy",
                "--affinity",
                str(selected),
                "--output",
                str(output),
                "--smoke",
            ]
        )
        == 0
    )
    manifest, suite = load_result(output)
    assert manifest["checks"][0]["contract"]["effective_affinity"] == str(selected)
    benchmark = suite.get_benchmark("format.layout/tiny/bibtex-tidy")
    assert all(
        run.get_metadata()["effective_affinity"] == str(selected) for run in benchmark.get_runs()
    )
    assert _current_cpus() == inherited
