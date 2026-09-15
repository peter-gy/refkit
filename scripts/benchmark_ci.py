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


def api_version(root: Path) -> int | None:
    path = root / "packages/refkit-bench/src/refkit_bench/api-version.json"
    if not path.is_file():
        return None
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict) or set(data) != {"api_version"}:
        raise ValueError(f"{path}: expected api_version")
    value = data["api_version"]
    if type(value) is not int or value < 1:
        raise ValueError(f"{path}: api_version must be a positive integer")
    return value


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
    target_root = Path(environment.get("CARGO_TARGET_DIR", str(output / "target"))).resolve()
    interpreters = {}
    try:
        versions = {"baseline": api_version(baseline), "candidate": api_version(candidate)}
        if versions["candidate"] is None:
            raise ValueError("candidate must declare its benchmark API version")
        comparable = versions["baseline"] == versions["candidate"]
        result.update(mode="comparison" if comparable else "absolute", api_versions=versions)
        roots = (
            [("baseline", baseline), ("candidate", candidate)]
            if comparable
            else [("candidate", candidate)]
        )
        for name, root in roots:
            venv = output / f"{name}-env"
            target = str(target_root / name)
            # Cargo's relative source paths and mtimes can alias across checkouts.
            env = {
                **environment,
                "UV_PROJECT_ENVIRONMENT": str(venv),
                "CARGO_TARGET_DIR": target,
                "CARGO_BUILD_BUILD_DIR": target,
            }
            # Selected revisions use the candidate harness and dependency lock.
            execute(
                [
                    "uv",
                    "sync",
                    "--project",
                    str(candidate),
                    "--locked",
                    "--python",
                    environment.get("UV_PYTHON", sys.executable),
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
        order = [name for name, _ in roots]
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
        command = [interpreters["candidate"], "-m", "refkit_bench.runner"]
        command += (
            ["compare", str(output / "baseline"), str(output / "candidate"), "--json"]
            if comparable
            else ["report", str(output / "candidate"), "--json"]
        )
        summary = execute(
            command,
            env=environment,
            capture=True,
        )
        result["comparison" if comparable else "measurement"] = json.loads(summary)
        result["status"] = "complete"
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
        description="Measure release builds with declared benchmark API comparability."
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
