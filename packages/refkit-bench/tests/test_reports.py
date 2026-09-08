from __future__ import annotations

import json
from hashlib import sha256
from pathlib import Path
from typing import Any

import pyperf
import pytest

from refkit_bench.report import compare, load_result, summarize
from refkit_bench.runner import main


def result(directory: Path, workers: list[list[float]], *, metadata=None) -> dict[str, Any]:
    directory.mkdir()
    name = "parse.bibtex/real/refkit"
    run_metadata = {
        "name": name,
        "unit": "second",
        "loops": 1,
        "artifact_sha256": "binary-and-wrappers",
        "case_sha256": "contract",
        "benchmark_sha256": "harness",
        "runtime": "Python",
        "build_mode": "release",
        "hostname": "host",
        "cpu_count": 4,
        **(metadata or {}),
    }
    runs = [pyperf.Run(values, metadata=run_metadata, collect_metadata=False) for values in workers]
    pyperf.BenchmarkSuite([pyperf.Benchmark(runs)]).dump(str(directory / "timings.json"))
    manifest = {
        "schema": 1,
        "status": "complete",
        "profile": "measurement",
        "environment": {
            "python": "3.12",
            "pyperf_version": "2.10.0",
            "implementation": "CPython",
            "platform": "test",
            "machine": "arm64",
            "processor": "arm",
            "hostname": "host",
            "cpu_count": 4,
            "cpu_model": "Model A",
            "benchmark_sha256": "harness",
        },
        "measurement": {"warmups": 10, "min_time": 0.1, "affinity": None},
        "checks": [
            {
                "name": name,
                "status": "ok",
                "case_sha256": "contract",
                "contract": {"input_sha256": "input", "style_sha256": "style"},
                "artifact": {
                    "artifact_sha256": "binary-and-wrappers",
                    "runtime": "Python",
                    "build_mode": "release",
                },
            }
        ],
        "timings_sha256": sha256((directory / "timings.json").read_bytes()).hexdigest(),
    }
    (directory / "manifest.json").write_text(json.dumps(manifest))
    return manifest


def test_summary_weights_independent_workers_equally(tmp_path: Path):
    result(tmp_path / "run", [[1.0] * 100, [9.0]])
    row = summarize(tmp_path / "run")["rows"][0]
    assert row["seconds"] == 5.0
    assert row["workers"] == 2
    assert row["samples"] == 101


def test_five_workers_report_times_ratios_and_uncertainty(tmp_path: Path, capsys):
    result(tmp_path / "before", [[2.0]] * 5)
    result(tmp_path / "after", [[0.003]] * 5)
    row = compare(tmp_path / "before", tmp_path / "after")["rows"][0]
    assert row["baseline_seconds"] == 2.0
    assert row["candidate_seconds"] == 0.003
    assert row["candidate_over_baseline"] == 0.0015
    assert row["ratio_interval_95"] == (0.0015, 0.0015)
    assert row["assessment"] == "interval below 1"
    assert main(["compare", str(tmp_path / "before"), str(tmp_path / "after")]) == 0
    output = capsys.readouterr().out
    assert "2 s" in output
    assert "3 ms" in output
    assert "interval below 1" in output
    assert "95% interval" in output


def test_four_workers_report_a_ratio_with_insufficient_uncertainty(tmp_path: Path, capsys):
    result(tmp_path / "before", [[1.0]] * 4)
    result(tmp_path / "after", [[2.0]] * 4)
    row = compare(tmp_path / "before", tmp_path / "after")["rows"][0]
    assert row["candidate_over_baseline"] == 2.0
    assert row["ratio_interval_95"] is None
    assert row["assessment"] == "insufficient workers"
    assert main(["compare", str(tmp_path / "before"), str(tmp_path / "after")]) == 0
    assert "insufficient workers" in capsys.readouterr().out


@pytest.mark.parametrize(
    "section,key,value",
    [
        ("environment", "cpu_model", "Model B"),
        ("measurement", "warmups", 1),
    ],
)
def test_changed_measurement_conditions_require_a_new_baseline(tmp_path: Path, section, key, value):
    result(tmp_path / "before", [[1.0]] * 5)
    manifest = result(tmp_path / "after", [[1.0]] * 5)
    manifest[section][key] = value
    (tmp_path / "after/manifest.json").write_text(json.dumps(manifest))
    with pytest.raises(ValueError):
        compare(tmp_path / "before", tmp_path / "after")


def test_changed_workload_cannot_be_compared(tmp_path: Path):
    result(tmp_path / "before", [[1.0]] * 5)
    manifest = result(tmp_path / "after", [[1.0]] * 5)
    manifest["checks"][0]["contract"]["input_sha256"] = "different-input"
    (tmp_path / "after/manifest.json").write_text(json.dumps(manifest))
    with pytest.raises(ValueError, match="contract differs"):
        compare(tmp_path / "before", tmp_path / "after")


def test_observed_cpu_affinity_must_match(tmp_path: Path):
    result(tmp_path / "before", [[1.0]] * 5, metadata={"cpu_affinity": "0"})
    result(tmp_path / "after", [[1.0]] * 5, metadata={"cpu_affinity": "1"})
    with pytest.raises(ValueError, match="hardware or affinity"):
        compare(tmp_path / "before", tmp_path / "after")


@pytest.mark.parametrize(
    "field,value",
    [
        ("artifact_sha256", "changed-wrapper"),
        ("benchmark_sha256", "changed-harness"),
        ("case_sha256", "changed-thread-budget"),
    ],
)
def test_observed_worker_identity_must_match_preflight(tmp_path: Path, field, value):
    result(tmp_path / "run", [[1.0]], metadata={field: value})
    with pytest.raises(ValueError):
        load_result(tmp_path / "run")


def test_timing_archive_integrity_is_checked(tmp_path: Path):
    result(tmp_path / "run", [[1.0]])
    with (tmp_path / "run/timings.json").open("a") as handle:
        handle.write(" ")
    with pytest.raises(ValueError, match="archive hash"):
        load_result(tmp_path / "run")


def test_workers_with_different_affinity_cannot_be_compared(tmp_path: Path):
    result(tmp_path / "before", [[1.0]] * 5)
    manifest = result(tmp_path / "after", [[1.0]] * 5)
    archive = tmp_path / "after/timings.json"
    suite = pyperf.BenchmarkSuite.load(str(archive))
    original = suite.get_benchmark("parse.bibtex/real/refkit").get_runs()
    runs = [
        pyperf.Run(
            run.values,
            metadata={**run.get_metadata(), "cpu_affinity": "0" if index == 0 else "1"},
            collect_metadata=False,
        )
        for index, run in enumerate(original)
    ]
    pyperf.BenchmarkSuite([pyperf.Benchmark(runs)]).dump(str(archive), replace=True)
    manifest["timings_sha256"] = sha256(archive.read_bytes()).hexdigest()
    (tmp_path / "after/manifest.json").write_text(json.dumps(manifest))
    with pytest.raises(ValueError, match="workers used different hardware or affinity"):
        compare(tmp_path / "before", tmp_path / "after")


def test_complete_archive_must_cover_every_validated_case(tmp_path: Path):
    manifest = result(tmp_path / "run", [[1.0]])
    manifest["checks"].append(
        {
            **manifest["checks"][0],
            "name": "inspect.keys/real/refkit",
        }
    )
    (tmp_path / "run/manifest.json").write_text(json.dumps(manifest))
    with pytest.raises(ValueError, match="timings do not cover the validated case set"):
        load_result(tmp_path / "run")
