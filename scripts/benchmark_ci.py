from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path


def execute(command: list[str], *, env: dict[str, str], capture: bool = False) -> str:
    sys.stderr.write(f"Running: {command}\n")
    result = subprocess.run(
        command,
        check=True,
        env=env,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        timeout=2700,
    )
    return result.stdout or ""


def run(candidate: Path, baseline: Path, output: Path, platform: str) -> int:
    output.mkdir(parents=True, exist_ok=False)
    revisions = {
        name: execute(
            ["git", "-C", str(root), "rev-parse", "HEAD"], env=dict(os.environ), capture=True
        ).strip()
        for name, root in (("baseline", baseline), ("candidate", candidate))
    }
    result: dict[str, object] = {
        "status": "failed",
        "baseline_sha": revisions["baseline"],
        "candidate_sha": revisions["candidate"],
        "platform": platform,
        "runner_image": os.environ.get("ImageVersion", "unknown"),
        "runner_arch": os.environ.get("RUNNER_ARCH", "unknown"),
    }
    environment = {
        **os.environ,
        "MATURIN_PEP517_ARGS": "--profile release --locked",
        "CARGO_BUILD_JOBS": "2",
    }
    interpreters = {}
    try:
        for name, root in (("baseline", baseline), ("candidate", candidate)):
            venv = output / f"{name}-env"
            env = {**environment, "UV_PROJECT_ENVIRONMENT": str(venv)}
            # Both installations use the candidate harness and dependency lock.
            execute(
                [
                    "uv",
                    "sync",
                    "--project",
                    str(candidate),
                    "--locked",
                    "--python",
                    sys.executable,
                    "--package",
                    "refkit-bench",
                    "--no-default-groups",
                    "--no-install-package",
                    "refkit",
                    "--no-install-package",
                    "polars-refkit",
                ],
                env=env,
            )
            python = venv / ("Scripts/python.exe" if os.name == "nt" else "bin/python")
            execute(
                [
                    "uv",
                    "pip",
                    "install",
                    "--python",
                    str(python),
                    "--no-deps",
                    str(root / "packages/refkit"),
                    str(root / "packages/polars-refkit"),
                ],
                env=env,
            )
            interpreters[name] = str(python)
        order = ["baseline", "candidate"]
        if int(os.environ.get("GITHUB_RUN_NUMBER", "0")) % 2:
            order.reverse()
        result["execution_order"] = order
        for name in order:
            execute(
                [
                    interpreters[name],
                    "-m",
                    "refkit_bench.runner",
                    "run",
                    "--lane",
                    "all",
                    "--dataset",
                    "real",
                    "--package",
                    "refkit",
                    "--package",
                    "polars-eager",
                    "--package",
                    "polars-lazy",
                    "--processes",
                    "5",
                    "--values",
                    "5",
                    "--warmups",
                    "5",
                    "--min-time",
                    "0.05",
                    "--seed",
                    "2026",
                    "--output",
                    str(output / name),
                ],
                env=environment,
            )
        comparison = execute(
            [
                interpreters["candidate"],
                "-m",
                "refkit_bench.runner",
                "compare",
                str(output / "baseline"),
                str(output / "candidate"),
                "--json",
            ],
            env=environment,
            capture=True,
        )
        result.update(status="complete", comparison=json.loads(comparison))
    except (subprocess.SubprocessError, OSError, ValueError) as exc:
        result["detail"] = str(exc)
        sys.stderr.write(f"Benchmark failed: {exc}\n")
    finally:
        (output / "comparison.json").write_text(
            json.dumps(result, indent=2) + "\n", encoding="utf-8"
        )
    return int(result["status"] != "complete")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Measure two release builds on the same CI runner."
    )
    parser.add_argument("--candidate", type=Path, required=True)
    parser.add_argument("--baseline", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--platform", choices=("Linux", "Windows", "macOS"), required=True)
    args = parser.parse_args()
    return run(
        args.candidate.resolve(), args.baseline.resolve(), args.output.resolve(), args.platform
    )


if __name__ == "__main__":
    raise SystemExit(main())
