from __future__ import annotations

import json
import random
from hashlib import sha256
from pathlib import Path
from statistics import mean, median, stdev
from typing import Any

import pyperf


def load_result(directory: Path) -> tuple[dict[str, Any], Any]:
    manifest = json.loads((directory / "manifest.json").read_text())
    if manifest.get("schema") != 1 or manifest.get("status") != "complete":
        raise ValueError(f"{directory}: expected a complete benchmark run with schema 1")
    if sha256((directory / "timings.json").read_bytes()).hexdigest() != manifest.get(
        "timings_sha256"
    ):
        raise ValueError(f"{directory}: timing archive hash differs from the run manifest")
    suite = pyperf.BenchmarkSuite.load(str(directory / "timings.json"))
    expected = {item["name"] for item in manifest["checks"] if item["status"] == "ok"}
    if len(expected) != len(manifest["checks"]):
        raise ValueError(f"{directory}: every selected case must have one successful validation")
    if set(suite.get_benchmark_names()) != expected:
        raise ValueError(f"{directory}: timings do not cover the validated case set")
    for benchmark in suite:
        check = next(item for item in manifest["checks"] if item["name"] == benchmark.get_name())
        validate_measurement(benchmark, check, manifest["environment"]["benchmark_sha256"])
    return manifest, suite


def validate_measurement(benchmark: Any, expected: dict[str, Any], source_hash: str) -> None:
    if benchmark.get_name() != expected["name"]:
        raise ValueError("measurement worker returned a different case")
    for run in benchmark.get_runs():
        observed = run.get_metadata()
        for key, value in expected["artifact"].items():
            if observed.get(key) != value:
                raise ValueError(f"measured artifact field {key} differs from preflight")
        if observed.get("benchmark_sha256") != source_hash:
            raise ValueError("benchmark sources changed during measurement")
        if observed.get("case_sha256") != expected["case_sha256"]:
            raise ValueError("timed worker contract differs from preflight")


def worker_means(benchmark: Any) -> list[float]:
    return [mean(run.values) for run in benchmark.get_runs() if run.values]


def _hardware(benchmark: Any) -> dict[str, object]:
    observations = [
        {
            key: run.get_metadata().get(key)
            for key in (
                "hostname",
                "cpu_count",
                "cpu_model_name",
                "cpu_affinity",
                "cpu_config",
                "python_compiler",
            )
        }
        for run in benchmark.get_runs()
        if run.values
    ]
    if any(item != observations[0] for item in observations):
        raise ValueError(
            f"{benchmark.get_name()}: workers used different hardware or affinity settings"
        )
    return observations[0]


def summarize(directory: Path) -> dict[str, Any]:
    manifest, suite = load_result(directory)
    rows = []
    for benchmark in suite:
        workers = worker_means(benchmark)
        rows.append(
            {
                "case": benchmark.get_name(),
                "seconds": median(workers),
                "workers": len(workers),
                "samples": benchmark.get_nvalue(),
                "worker_cv": stdev(workers) / mean(workers) if len(workers) > 1 else None,
                "minimum_worker_mean": min(workers),
                "maximum_worker_mean": max(workers),
            }
        )
    return {
        "profile": manifest["profile"],
        "statistic": "median of worker means, seconds per complete operation",
        "rows": sorted(rows, key=lambda row: row["case"]),
    }


def _quantile(values: list[float], probability: float) -> float:
    position = (len(values) - 1) * probability
    left = int(position)
    right = min(left + 1, len(values) - 1)
    return values[left] + (values[right] - values[left]) * (position - left)


def ratio_interval(
    baseline: list[float], candidate: list[float], *, seed: int = 2026, resamples: int = 10_000
) -> tuple[float, float] | None:
    if min(len(baseline), len(candidate)) < 5:
        return None
    rng = random.Random(seed)
    ratios = sorted(
        median(rng.choices(candidate, k=len(candidate)))
        / median(rng.choices(baseline, k=len(baseline)))
        for _ in range(resamples)
    )
    return _quantile(ratios, 0.025), _quantile(ratios, 0.975)


def compare(baseline: Path, candidate: Path) -> dict[str, Any]:
    before, old_suite = load_result(baseline)
    after, new_suite = load_result(candidate)
    if before["profile"] != "measurement" or after["profile"] != "measurement":
        raise ValueError("smoke runs verify execution and cannot support performance comparisons")
    for field in (
        "python",
        "pyperf_version",
        "implementation",
        "platform",
        "machine",
        "processor",
        "hostname",
        "cpu_count",
        "cpu_model",
        "benchmark_sha256",
    ):
        if before["environment"][field] != after["environment"][field]:
            raise ValueError(f"comparison requires matching {field}")
    for field in ("warmups", "min_time", "affinity"):
        if before["measurement"][field] != after["measurement"][field]:
            raise ValueError(f"comparison requires matching measurement {field}")
    old_checks = {row["name"]: row for row in before["checks"]}
    new_checks = {row["name"]: row for row in after["checks"]}
    if old_checks.keys() != new_checks.keys():
        raise ValueError(
            "comparison requires identical case sets. "
            "Select matching lanes, datasets, and participants"
        )
    rows = []
    for name in sorted(old_checks):
        old = old_checks[name]
        new = new_checks[name]
        if old["contract"] != new["contract"]:
            raise ValueError(f"{name}: input, options, style, or operation contract differs")
        for field in ("runtime", "build_mode", "python_flags", "node_version", "v8_version"):
            if old["artifact"].get(field) != new["artifact"].get(field):
                raise ValueError(f"{name}: runtime setting {field} differs")
        old_workers = worker_means(old_suite.get_benchmark(name))
        new_workers = worker_means(new_suite.get_benchmark(name))
        if _hardware(old_suite.get_benchmark(name)) != _hardware(new_suite.get_benchmark(name)):
            raise ValueError(f"{name}: measured hardware or affinity settings differ")
        interval = ratio_interval(old_workers, new_workers)
        rows.append(
            {
                "case": name,
                "baseline_seconds": median(old_workers),
                "candidate_seconds": median(new_workers),
                "candidate_over_baseline": median(new_workers) / median(old_workers),
                "ratio_interval_95": interval,
                "baseline_workers": len(old_workers),
                "candidate_workers": len(new_workers),
                "assessment": "insufficient workers"
                if interval is None
                else "interval below 1"
                if interval[1] < 1
                else "interval above 1"
                if interval[0] > 1
                else "interval includes 1",
            }
        )
    return {
        "ratio": "candidate / baseline. Below 1 is lower elapsed time.",
        "interval": (
            "95% percentile bootstrap resampling independent workers within each run. "
            "Exploratory per-case intervals with no multiple-comparison adjustment."
        ),
        "rows": rows,
    }
