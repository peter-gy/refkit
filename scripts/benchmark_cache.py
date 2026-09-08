from __future__ import annotations

import argparse
import io
import json
import os
import re
import subprocess
import sys
import zipfile
from typing import Any
from urllib.parse import urlencode

from scripts.benchmark_report import validate, validate_evidence


def api(route: str) -> Any:
    result = subprocess.run(
        ["gh", "api", route], capture_output=True, text=True, check=True, timeout=60
    )
    return json.loads(result.stdout)


def download_report(repository: str, artifact: dict[str, Any]) -> dict[str, Any]:
    if artifact["expired"] or artifact["size_in_bytes"] > 10_000_000:
        raise ValueError("benchmark artifact is expired or exceeds 10 MB")
    archive = subprocess.run(
        ["gh", "api", f"repos/{repository}/actions/artifacts/{artifact['id']}/zip"],
        capture_output=True,
        check=True,
        timeout=60,
    ).stdout
    with zipfile.ZipFile(io.BytesIO(archive)) as bundle:
        info = bundle.getinfo("report.json")
        if info.file_size > 200_000:
            raise ValueError("benchmark report exceeds 200 KB")
        report = json.loads(bundle.read(info))
        validate(report)
        if all(item["status"] == "complete" for item in report["platforms"].values()):
            if sum(item.file_size for item in bundle.infolist()) > 50_000_000:
                raise ValueError("uncompressed benchmark evidence exceeds 50 MB")
            validate_evidence(report, bundle.read)
    return report


def find(
    repository: str, fingerprint: str, baseline: str | None = None, *, candidate_only: bool = False
) -> str:
    name = f"benchmark-results-{fingerprint}"
    for page in range(1, 11):
        artifacts = api(
            f"repos/{repository}/actions/artifacts?"
            + urlencode({"name": name, "per_page": 100, "page": page})
        )["artifacts"]
        for artifact in artifacts:
            if artifact["name"] != name or artifact["expired"]:
                continue
            run_id = artifact["workflow_run"]["id"]
            run = api(f"repos/{repository}/actions/runs/{run_id}")
            trusted = (
                run["path"] == ".github/workflows/benchmarks.yml" and run["head_branch"] == "main"
            ) or (
                run["path"] == ".github/workflows/publish.yml"
                and re.fullmatch(r"v\d+\.\d+\.\d+(?:-rc\.\d+)?", run["head_branch"]) is not None
            )
            if (
                not trusted
                or run["event"] != "push"
                or run["head_repository"]["full_name"] != repository
            ):
                continue
            try:
                report = download_report(repository, artifact)
                if report.get("inputs_sha256") == fingerprint and all(
                    item["status"] == "complete" for item in report["platforms"].values()
                ):
                    if (
                        baseline is None
                        or baseline == fingerprint
                        or candidate_only
                        or report.get("baseline_inputs_sha256") == baseline
                    ):
                        return str(run_id)
            except (
                ValueError,
                KeyError,
                TypeError,
                zipfile.BadZipFile,
                subprocess.CalledProcessError,
            ) as exc:
                sys.stderr.write(
                    f"Skipping unavailable benchmark artifact {artifact['id']}: {exc}\n"
                )
        if len(artifacts) < 100:
            break
    return ""


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Find complete benchmark evidence with matching inputs."
    )
    parser.add_argument("fingerprint")
    args = parser.parse_args()
    if re.fullmatch(r"[0-9a-f]{64}", args.fingerprint) is None:
        parser.error("expected a SHA-256 fingerprint")
    run_id = find(
        os.environ["GITHUB_REPOSITORY"],
        args.fingerprint,
        os.environ.get("BASELINE_INPUTS_SHA256"),
        candidate_only=os.environ.get("CANDIDATE_ONLY") == "true",
    )
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as handle:
        handle.write(f"run-id={run_id}\n")


if __name__ == "__main__":
    main()
