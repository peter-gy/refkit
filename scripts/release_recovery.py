"""Validate retained tag-run artifacts before completing npm publication."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any


def validate(
    run: dict[str, Any],
    jobs: list[dict[str, Any]],
    artifacts: list[dict[str, Any]],
    tag: str,
    commit: str,
) -> str:
    if (
        run.get("event") != "push"
        or run.get("path") != ".github/workflows/publish.yml"
        or run.get("head_branch") != tag
        or run.get("head_sha") != commit
        or run.get("status") != "completed"
    ):
        raise ValueError("source run must be a completed Publish run for the requested tag commit")

    states = {
        job["name"]: job.get("conclusion")
        for job in sorted(jobs, key=lambda job: job.get("run_attempt", 1))
    }
    required = {
        "refkit / Publish to PyPI",
        "polars-refkit / Publish to PyPI",
        "refkit-js artifacts / Build npm package",
        "refkit-js artifacts / Browser package",
        "Benchmark evidence / results / Consolidated results",
    }
    required.update(
        f"refkit-js artifacts / Node {node} / {runner}"
        for node in ("22.19.0", "24", "26")
        for runner in ("ubuntu-latest", "macos-latest", "windows-latest")
    )
    required.update(name for name in states if name.startswith("refkit-js artifacts /"))
    failed = sorted(name for name in required if states.get(name) != "success")
    if failed:
        raise ValueError(f"source release gates must succeed: {', '.join(failed)}")

    names = [artifact["name"] for artifact in artifacts if not artifact.get("expired", True)]
    benchmarks = [name for name in names if re.fullmatch(r"benchmark-results-[a-f0-9]{64}", name)]
    if names.count("refkit-js-npm") != 1 or len(benchmarks) != 1:
        raise ValueError("source run must retain one npm artifact and one benchmark artifact")
    return benchmarks[0]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--jobs", type=Path, required=True)
    parser.add_argument("--artifacts", type=Path, required=True)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--commit", required=True)
    args = parser.parse_args()
    run = json.loads(args.run.read_text())
    jobs = [job for page in json.loads(args.jobs.read_text()) for job in page["jobs"]]
    artifacts = [
        item for page in json.loads(args.artifacts.read_text()) for item in page["artifacts"]
    ]
    try:
        name = validate(run, jobs, artifacts, args.tag, args.commit)
    except ValueError as error:
        raise SystemExit(str(error)) from error
    sys.stdout.write(f"{name}\n")


if __name__ == "__main__":
    main()
