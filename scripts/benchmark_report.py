from __future__ import annotations

import argparse
import json
import math
import os
from collections.abc import Callable
from hashlib import sha256
from pathlib import Path
from typing import Any

PLATFORMS = ("Linux", "Windows", "macOS")
THRESHOLD = 0.05


def assessment(row: dict[str, Any]) -> str:
    low, high = row["ratio_interval_95"]
    if high < 1 - THRESHOLD:
        return "faster"
    if low > 1 + THRESHOLD:
        return "slower"
    if low >= 1 - THRESHOLD and high <= 1 + THRESHOLD:
        return "same"
    return "inconclusive"


def validate(report: dict[str, Any]) -> None:
    if report["schema"] != 1 or report["threshold"] != THRESHOLD:
        raise ValueError("unsupported CI benchmark schema or threshold")
    for key in ("baseline_sha", "candidate_sha"):
        value = report[key]
        if len(value) != 40 or any(char not in "0123456789abcdef" for char in value):
            raise ValueError(f"invalid {key}")
    if request := report.get("request"):
        for key in ("candidate_sha", "baseline_sha"):
            value = request[key]
            if len(value) != 40 or any(char not in "0123456789abcdef" for char in value):
                raise ValueError(f"invalid request {key}")
        if type(request["reused"]) is not bool or type(request["unchanged"]) is not bool:
            raise ValueError("invalid reuse status")
    if set(report["platforms"]) != set(PLATFORMS):
        raise ValueError("expected Linux, Windows, and macOS results")
    for platform in report["platforms"].values():
        if platform["status"] not in {"complete", "failed", "unmeasured"}:
            raise ValueError("invalid platform status")
        if platform["status"] != "complete":
            continue
        rows = platform["comparison"]["rows"]
        if not rows or len(rows) > 100:
            raise ValueError("expected 1 to 100 comparison rows")
        names = set()
        for row in rows:
            case = row["case"]
            if (
                not isinstance(case, str)
                or len(case) > 150
                or any(char not in "abcdefghijklmnopqrstuvwxyz0123456789./_-" for char in case)
                or case in names
            ):
                raise ValueError("invalid or duplicate benchmark case")
            names.add(case)
            for key in ("baseline_seconds", "candidate_seconds", "candidate_over_baseline"):
                number = row[key]
                if type(number) not in (int, float) or not math.isfinite(number) or number <= 0:
                    raise ValueError(f"invalid {key}")
            interval = row["ratio_interval_95"]
            if (
                not isinstance(interval, (list, tuple))
                or len(interval) != 2
                or any(
                    type(n) not in (int, float) or not math.isfinite(n) or n <= 0 for n in interval
                )
                or interval[0] > interval[1]
            ):
                raise ValueError("expected a finite 95% ratio interval")
            for key in ("baseline_workers", "candidate_workers"):
                if type(row[key]) is not int or row[key] < 5:
                    raise ValueError("comparison requires at least five workers")
            if not math.isclose(
                row["candidate_over_baseline"],
                row["candidate_seconds"] / row["baseline_seconds"],
            ):
                raise ValueError("ratio differs from elapsed times")


def validate_evidence(report: dict[str, Any], read: Callable[[str], bytes]) -> None:
    for name in PLATFORMS:
        expected = {row["case"] for row in report["platforms"][name]["comparison"]["rows"]}
        for revision in ("baseline", "candidate"):
            prefix = f"{name}/{revision}"
            manifest = json.loads(read(f"{prefix}/manifest.json"))
            if (
                manifest["status"] != "complete"
                or manifest["profile"] != "measurement"
                or manifest["timings_sha256"] != sha256(read(f"{prefix}/timings.json")).hexdigest()
                or {row["name"] for row in manifest["checks"] if row["status"] == "ok"} != expected
                or any(row["status"] != "ok" for row in manifest["checks"])
            ):
                raise ValueError(f"{prefix}: incomplete or inconsistent raw benchmark evidence")


def markdown(report: dict[str, Any]) -> str:
    validate(report)
    if all(item["status"] == "unmeasured" for item in report["platforms"].values()):
        return (
            "## Benchmark results\n\n"
            f"Commit `{report['candidate_sha'][:12]}` has unchanged runtime, build, dependency, "
            f"and benchmark inputs relative to `{report['baseline_sha'][:12]}`.\n\n"
            "Performance: unchanged inputs. Saved timing evidence is unavailable. "
            "Release publication collects measurements when needed.\n"
        )
    request = report.get("request", {})
    if request.get("reused") and not request.get("comparison_reused"):
        return reused_markdown(report)
    lines = [
        "## Benchmark results",
        "",
        f"Base `{report['baseline_sha'][:12]}` → candidate `{report['candidate_sha'][:12]}`.",
        "",
        "Elapsed time per complete operation. Negative changes mean faster execution.",
        "Each OS measures both revisions on one runner with a shared harness and dependencies.",
        "",
        "| Platform | Faster | Slower | Same | Inconclusive |",
        "| --- | ---: | ---: | ---: | ---: |",
    ]
    if request.get("reused"):
        lines.insert(
            2, f"Commit `{request['candidate_sha'][:12]}` reuses this measured comparison.\n"
        )
    for name in PLATFORMS:
        platform = report["platforms"][name]
        if platform["status"] == "failed":
            lines.append(f"| {name} | Measurement failed | | | |")
            continue
        states = [assessment(row) for row in platform["comparison"]["rows"]]
        counts = [
            str(states.count(state)) for state in ("faster", "slower", "same", "inconclusive")
        ]
        lines.append(f"| {name} | {' | '.join(counts)} |")
    lines.extend(
        [
            "",
            "Faster/slower requires the entire 95% bootstrap interval to exceed the ±5% band.",
            "Same means the interval is within ±5%. Other intervals are inconclusive.",
            "Hosted-runner measurements are advisory. Repeat close results on controlled hardware.",
            "Intervals are per case, with no multiple-comparison adjustment.",
        ]
    )
    for name in PLATFORMS:
        platform = report["platforms"][name]
        lines.extend(["", f"### {name}", ""])
        if platform["status"] == "failed":
            lines.append(
                "Measurement or comparison failed. Inspect this run's logs and raw artifacts."
            )
            continue
        lines.extend(
            [
                "| Case | Base (ms) | Candidate (ms) | Change | 95% change interval | Result |",
                "| --- | ---: | ---: | ---: | ---: | --- |",
            ]
        )
        for row in platform["comparison"]["rows"]:
            low, high = row["ratio_interval_95"]
            delta = (row["candidate_over_baseline"] - 1) * 100
            lines.append(
                f"| `{row['case']}` | {row['baseline_seconds'] * 1000:.4g} "
                f"| {row['candidate_seconds'] * 1000:.4g} | {delta:+.1f}% "
                f"| {(low - 1) * 100:+.1f}% to {(high - 1) * 100:+.1f}% "
                f"| {assessment(row)} |"
            )
    return "\n".join(lines) + "\n"


def reused_markdown(report: dict[str, Any]) -> str:
    request = report["request"]
    lines = [
        "## Benchmark results",
        "",
        f"Commit `{request['candidate_sha'][:12]}` reuses measurements from "
        f"`{report['candidate_sha'][:12]}`.",
        "",
        "Runtime, build, dependency, and benchmark inputs match the saved measurements.",
        "The recorded native artifacts and measurement environment remain in the result archive.",
        "",
        "Performance: unchanged inputs relative to the previous commit. Timings are reused."
        if request["unchanged"]
        else "Saved absolute timings. This commit transition requires a fresh comparison.",
        "",
        "| Platform | Case | Elapsed (ms) | Workers |",
        "| --- | --- | ---: | ---: |",
    ]
    for name in PLATFORMS:
        lines.extend(
            f"| {name} | `{row['case']}` | {row['candidate_seconds'] * 1000:.4g} "
            f"| {row['candidate_workers']} |"
            for row in report["platforms"][name]["comparison"]["rows"]
        )
    return "\n".join(lines) + "\n"


def consolidate(directory: Path, baseline: str, candidate: str) -> dict[str, Any]:
    platforms = {}
    for name in PLATFORMS:
        path = directory / name / "comparison.json"
        platform = (
            json.loads(path.read_text(encoding="utf-8")) if path.is_file() else {"status": "failed"}
        )
        if platform.get("baseline_sha") != baseline or platform.get("candidate_sha") != candidate:
            platform = {"status": "failed"}
        platforms[name] = platform
    report = {
        "schema": 1,
        "threshold": THRESHOLD,
        "baseline_sha": baseline,
        "candidate_sha": candidate,
        "platforms": platforms,
    }
    validate(report)
    return report


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Consolidate benchmark shards into JSON and Markdown."
    )
    parser.add_argument("directory", type=Path)
    parser.add_argument("--baseline", required=True)
    parser.add_argument("--candidate", required=True)
    args = parser.parse_args()
    reused = os.environ.get("REUSED_RUN", "")
    fingerprint = os.environ.get("INPUTS_SHA256")
    if reused:
        report = json.loads((args.directory / "report.json").read_text(encoding="utf-8"))
        validate(report)
        if report.get("inputs_sha256") != fingerprint or any(
            item["status"] != "complete" for item in report["platforms"].values()
        ):
            raise ValueError("reused evidence must be complete and match the requested inputs")
        validate_evidence(report, lambda name: (args.directory / name).read_bytes())
    else:
        report = consolidate(args.directory, args.baseline, args.candidate)
        if os.environ.get("UNCHANGED_ONLY") == "true":
            report["platforms"] = {name: {"status": "unmeasured"} for name in PLATFORMS}
        report["inputs_sha256"] = fingerprint
        report["baseline_inputs_sha256"] = os.environ.get("BASELINE_INPUTS_SHA256")
        report["measurement_run_id"] = os.environ.get("GITHUB_RUN_ID")
    report["request"] = {
        "candidate_sha": args.candidate,
        "baseline_sha": args.baseline,
        "reused": bool(reused),
        "unchanged": bool(fingerprint) and fingerprint == os.environ.get("BASELINE_INPUTS_SHA256"),
        "source_run_id": reused or os.environ.get("GITHUB_RUN_ID"),
        "comparison_reused": bool(reused)
        and bool(fingerprint)
        and report.get("baseline_inputs_sha256") == os.environ.get("BASELINE_INPUTS_SHA256")
        and fingerprint != os.environ.get("BASELINE_INPUTS_SHA256"),
    }
    args.directory.mkdir(parents=True, exist_ok=True)
    (args.directory / "report.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    summary = markdown(report)
    (args.directory / "summary.md").write_text(summary, encoding="utf-8")
    if destination := os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(destination, "a", encoding="utf-8") as handle:
            handle.write(summary)
    return int(any(item["status"] == "failed" for item in report["platforms"].values()))


if __name__ == "__main__":
    raise SystemExit(main())
