from __future__ import annotations

import argparse
import json
import shutil
import subprocess
from pathlib import Path

from scripts.benchmark_inputs import fingerprint
from scripts.benchmark_report import markdown, validate, validate_evidence


def package(directory: Path, output: Path, identity: str, candidate: str) -> None:
    report = json.loads((directory / "report.json").read_text(encoding="utf-8"))
    validate(report)
    if any(item["status"] != "complete" for item in report["platforms"].values()):
        raise ValueError("release benchmark evidence must cover all three platforms")
    if (
        report.get("inputs_sha256") != identity
        or report.get("request", report)["candidate_sha"] != candidate
    ):
        raise ValueError("benchmark evidence differs from the release inputs or commit")
    validate_evidence(report, lambda name: (directory / name).read_bytes())
    output.mkdir(parents=True, exist_ok=True)
    (output / "benchmark-results.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    (output / "benchmark-results.md").write_text(markdown(report), encoding="utf-8")
    shutil.make_archive(str(output / "benchmark-results"), "zip", directory)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Package complete benchmark evidence for release assets."
    )
    parser.add_argument("directory", type=Path)
    parser.add_argument("--output", type=Path, default=Path.cwd())
    args = parser.parse_args()
    candidate = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    package(args.directory, args.output, fingerprint(Path.cwd(), candidate), candidate)


if __name__ == "__main__":
    main()
