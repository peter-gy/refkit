from __future__ import annotations

import json
import os
import subprocess
import sys
import zipfile
from pathlib import Path
from typing import Any
from urllib.parse import urlencode

from scripts.benchmark_cache import download_report
from scripts.benchmark_report import markdown

MARKER = "<!-- refkit-benchmarks -->"


def api(route: str, *, method: str = "GET", body: dict[str, Any] | None = None) -> Any:
    command = ["gh", "api", route, "--method", method]
    if body is not None:
        command += ["--input", "-"]
    response = subprocess.run(
        command,
        input=json.dumps(body) if body is not None else None,
        capture_output=True,
        text=True,
        check=True,
        timeout=60,
    )
    return json.loads(response.stdout)


def pages(route: str) -> list[dict[str, Any]]:
    rows = []
    for page in range(1, 101):
        separator = "&" if "?" in route else "?"
        batch = api(f"{route}{separator}per_page=100&page={page}")
        rows.extend(batch)
        if len(batch) < 100:
            return rows
    raise ValueError("GitHub pagination exceeded 10,000 records")


def read_report(repository: str, run: dict[str, Any]) -> dict[str, Any]:
    artifacts = api(f"repos/{repository}/actions/runs/{run['id']}/artifacts?per_page=100")[
        "artifacts"
    ]
    matches = [item for item in artifacts if item["name"].startswith("benchmark-results-")]
    if len(matches) != 1:
        raise ValueError("expected one consolidated benchmark artifact")
    return download_report(repository, matches[0])


def publish(repository: str, run: dict[str, Any]) -> None:
    if (
        run["event"] != "push"
        or run["head_branch"] != "main"
        or run["path"] != ".github/workflows/benchmarks.yml"
        or run["head_repository"]["full_name"] != repository
    ):
        return
    latest = api(
        f"repos/{repository}/actions/workflows/benchmarks.yml/runs?"
        + urlencode({"event": "push", "head_sha": run["head_sha"], "per_page": 100})
    )["workflow_runs"]
    if any(
        item["id"] > run["id"]
        or (item["id"] == run["id"] and item["run_attempt"] > run["run_attempt"])
        for item in latest
    ):
        return
    url = f"https://github.com/{repository}/actions/runs/{run['id']}/attempts/{run['run_attempt']}"
    try:
        report = read_report(repository, run)
        if report.get("request", report)["candidate_sha"] != run["head_sha"]:
            raise ValueError("report candidate differs from the measured commit")
        summary = markdown(report)
    except (ValueError, KeyError, TypeError, zipfile.BadZipFile) as exc:
        sys.stderr.write(f"Benchmark report unavailable: {exc}\n")
        summary = (
            "## Benchmark results\n\nMeasurements are incomplete. Inspect the workflow logs.\n"
        )
    body = f"{MARKER}\n{summary}\n[Workflow, logs, and consolidated artifact]({url})\n"
    route = f"repos/{repository}/commits/{run['head_sha']}/comments"
    comments = pages(route)
    existing = next(
        (
            comment
            for comment in comments
            if comment["user"]["login"] == "github-actions[bot]"
            and comment["body"].startswith(MARKER)
        ),
        None,
    )
    if existing:
        api(f"repos/{repository}/comments/{existing['id']}", method="PATCH", body={"body": body})
    else:
        api(route, method="POST", body={"body": body})


def main() -> None:
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text(encoding="utf-8"))
    publish(os.environ["GITHUB_REPOSITORY"], event["workflow_run"])


if __name__ == "__main__":
    main()
