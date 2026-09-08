from __future__ import annotations

import importlib
import json
import os
import platform
import shutil
import subprocess
import sys
from hashlib import sha256
from importlib.metadata import PackageNotFoundError, distribution, distributions, version
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
SOURCE = Path(__file__).parent


def source_digest() -> str:
    digest = sha256()
    for path in sorted(SOURCE.rglob("*")):
        if path.is_file() and path.suffix in {".py", ".mjs", ".json", ".csl"}:
            digest.update(path.relative_to(SOURCE).as_posix().encode())
            digest.update(path.read_bytes())
    return digest.hexdigest()


def environment() -> dict[str, object]:
    commit = "unknown"
    changed: bool | None = None
    if (
        SOURCE.resolve() == (ROOT / "packages/refkit-bench/src/refkit_bench").resolve()
        and shutil.which("git") is not None
    ):
        git = subprocess.run(
            ["git", "-C", str(ROOT), "rev-parse", "HEAD"], capture_output=True, text=True
        )
        dirty = subprocess.run(
            ["git", "-C", str(ROOT), "status", "--porcelain"], capture_output=True, text=True
        )
        if git.returncode == 0:
            commit = git.stdout.strip()
            changed = bool(dirty.stdout) if dirty.returncode == 0 else None
    return {
        "python": platform.python_version(),
        "pyperf_version": version("pyperf"),
        "implementation": platform.python_implementation(),
        "executable": sys.executable,
        "platform": platform.platform(),
        "machine": platform.machine(),
        "processor": platform.processor() or platform.machine(),
        "hostname": platform.node(),
        "cpu_count": os.cpu_count(),
        "cpu_model": cpu_model(),
        "runner_commit": commit,
        "runner_dirty": changed,
        "benchmark_sha256": source_digest(),
        "packages": {item.metadata["Name"]: item.version for item in distributions()},
    }


def cpu_model() -> str:
    if sys.platform == "darwin":
        result = subprocess.run(
            ["sysctl", "-n", "machdep.cpu.brand_string"], capture_output=True, text=True
        )
        return result.stdout.strip() or "unknown"
    cpuinfo = Path("/proc/cpuinfo")
    if cpuinfo.is_file():
        for line in cpuinfo.read_text().splitlines():
            key, _, value = line.partition(":")
            if key.strip() in {"model name", "Hardware"}:
                return value.strip()
    return platform.processor() or "unknown"


def artifact(package: str) -> dict[str, str]:
    name = "polars-refkit" if package.startswith("polars-") else package
    if name == "bibtex-tidy":
        return {}
    installed = distribution(name)
    module_name = {
        "refkit": "refkit._native",
        "polars-refkit": "polars_refkit._internal",
        "citeproc-py": "citeproc",
    }.get(name, name)
    module = importlib.import_module(module_name)
    if module.__file__ is None:
        raise RuntimeError(f"{name} has no inspectable module artifact")
    path = Path(module.__file__).resolve()
    digest = sha256()
    paths = sorted({path, *path.parent.rglob("*.py")})
    for source in paths:
        digest.update(source.relative_to(path.parent).as_posix().encode())
        digest.update(source.read_bytes())
    if name == "polars-refkit":
        for dependency in (
            "polars",
            "polars-runtime-32",
            "polars-runtime-64",
            "polars-runtime-compat",
        ):
            try:
                installed_dependency = distribution(dependency)
            except PackageNotFoundError:
                continue
            for source in sorted(installed_dependency.files or []):
                if source.suffix in {".py", ".so", ".pyd"}:
                    digest.update(f"{dependency}/{source}".encode())
                    digest.update(Path(str(installed_dependency.locate_file(source))).read_bytes())
    direct = json.loads(installed.read_text("direct_url.json") or "{}")
    return {
        "package_version": installed.version,
        "artifact_path": str(path),
        "artifact_sha256": digest.hexdigest(),
        "artifact_source_url": direct.get("url", "unknown"),
        "artifact_source_revision": direct.get("vcs_info", {}).get("commit_id", "unknown"),
        "build_mode": str(getattr(module, "build_mode", "unknown"))
        if name in {"refkit", "polars-refkit"}
        else "python",
        "runtime": f"{platform.python_implementation()} {platform.python_version()}",
        "python_flags": json.dumps(
            {
                "optimize": sys.flags.optimize,
                "dev_mode": sys.flags.dev_mode,
                "utf8_mode": sys.flags.utf8_mode,
            },
            sort_keys=True,
        ),
        "clock": "time.perf_counter",
    }
