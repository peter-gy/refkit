from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
from pathlib import Path


def included(path: str) -> bool:
    if Path(path).name in {"README.md", "AGENTS.md", "py.typed"} or Path(path).suffix == ".pyi":
        return False
    if any(part in {"tests", "benches"} for part in Path(path).parts):
        return False
    exact = {
        "Cargo.toml",
        "Cargo.lock",
        "pyproject.toml",
        "uv.lock",
        ".python-version",
        "rust-toolchain",
        "rust-toolchain.toml",
        ".github/workflows/benchmarks.yml",
        ".github/workflows/benchmark-results.yml",
        "scripts/benchmark_ci.py",
        "scripts/benchmark_report.py",
        "scripts/benchmark_inputs.py",
        "scripts/benchmark_cache.py",
    }
    if path in exact or path.startswith((".cargo/", ".github/actions/configure-rust-build/")):
        return True
    if path.startswith("crates/"):
        return Path(path).suffix != ".md"
    for package, trees in {
        "refkit": ("src/", "rust/"),
        "polars-refkit": ("polars_refkit/", "rust/"),
        "refkit-bench": ("src/",),
    }.items():
        prefix = f"packages/{package}/"
        if path.startswith(prefix):
            relative = path.removeprefix(prefix)
            return relative in {
                "pyproject.toml",
                "build_backend.py",
                "hatch_build.py",
            } or relative.startswith(trees)
    return False


def fingerprint(root: Path, revision: str) -> str:
    entries = subprocess.run(
        ["git", "-C", str(root), "ls-tree", "-rz", "--full-tree", revision],
        check=True,
        capture_output=True,
    ).stdout.split(b"\0")
    selected = [entry for entry in entries if entry and included(entry.split(b"\t", 1)[1].decode())]
    if not selected:
        raise ValueError(f"{revision}: benchmark inputs are empty")
    return hashlib.sha256(b"\0".join(sorted(selected))).hexdigest()


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Fingerprint tracked runtime, build, and benchmark inputs."
    )
    parser.add_argument("--baseline", required=True)
    parser.add_argument("--candidate", default="HEAD")
    args = parser.parse_args()
    root = Path.cwd()
    candidate = subprocess.check_output(["git", "rev-parse", args.candidate], text=True).strip()
    values = {
        "candidate": candidate,
        "baseline": args.baseline,
        "fingerprint": fingerprint(root, candidate),
        "baseline-fingerprint": fingerprint(root, args.baseline),
    }
    if destination := os.environ.get("GITHUB_OUTPUT"):
        with open(destination, "a", encoding="utf-8") as handle:
            handle.writelines(f"{key}={value}\n" for key, value in values.items())
    print(json.dumps(values, indent=2))  # noqa: T201


if __name__ == "__main__":
    main()
