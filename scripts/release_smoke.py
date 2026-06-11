from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
import tomllib
from pathlib import Path

PACKAGE = "refkit"
ROOT = Path(__file__).resolve().parents[1]
DEFAULT_PYTHONS = ("3.11", "3.12", "3.13", "3.14")


def main() -> int:
    parser = argparse.ArgumentParser(description="Import the built refkit wheel in fresh venvs.")
    parser.add_argument("--dist-dir", default="dist")
    parser.add_argument("--wheel")
    parser.add_argument("--pythons", nargs="+", default=list(DEFAULT_PYTHONS))
    parser.add_argument(
        "--allow-missing",
        action="store_true",
        help="Report missing Python interpreters without failing.",
    )
    args = parser.parse_args()

    version = project_version()
    wheel = Path(args.wheel) if args.wheel else find_wheel(Path(args.dist_dir), version)
    results = []
    with tempfile.TemporaryDirectory(prefix="refkit-release-smoke-") as temp:
        root = Path(temp)
        for python_spec in args.pythons:
            found = find_python(python_spec)
            if found is None:
                result = {
                    "python": python_spec,
                    "status": "missing",
                    "detail": f"uv python find {python_spec} failed",
                }
                results.append(result)
                if not args.allow_missing:
                    print(json.dumps(results, indent=2, sort_keys=True))
                    return 2
                continue
            venv = root / f"py{python_spec.replace('.', '')}"
            run(["uv", "venv", "--seed", "--python", str(found), str(venv)])
            venv_python = venv / "bin" / "python"
            run(["uv", "pip", "install", "--python", str(venv_python), str(wheel)])
            smoke = run(
                [
                    str(venv_python),
                    "-c",
                    SMOKE_CODE,
                ],
                cwd=root,
                clean_python_env=True,
                env={
                    "REFKIT_EXPECTED_PREFIX": str(venv),
                    "REFKIT_EXPECTED_VERSION": version,
                },
            )
            payload = json.loads(smoke.stdout)
            results.append({"python": python_spec, "status": "ok", **payload})

    print(json.dumps(results, indent=2, sort_keys=True))
    return 0


SMOKE_CODE = r"""
import importlib.metadata as metadata
import json
import os
from pathlib import Path
import sys

import refkit
import refkit._native as native

expected = os.environ["REFKIT_EXPECTED_VERSION"]
prefix = Path(os.environ["REFKIT_EXPECTED_PREFIX"]).resolve()
versions = {
    "refkit": refkit.__version__,
    "native": native.__version__,
    "metadata": metadata.version("refkit"),
}
if set(versions.values()) != {expected}:
    raise SystemExit(f"version mismatch: {versions}")

refkit_file = Path(refkit.__file__).resolve()
native_file = Path(native.__file__).resolve()
for label, path in {"refkit": refkit_file, "native": native_file}.items():
    if not path.is_relative_to(prefix):
        raise SystemExit(f"{label} imported from outside smoke venv: {path}")

print(json.dumps({
    "executable": sys.executable,
    "native_file": str(native_file),
    "refkit_file": str(refkit_file),
    "runtime": ".".join(map(str, sys.version_info[:3])),
    "version": expected,
}))
"""


def find_wheel(dist_dir: Path, version: str) -> Path:
    wheels = sorted(dist_dir.glob(f"{PACKAGE}-{version}-*.whl"))
    if not wheels:
        raise SystemExit(f"missing wheel: {dist_dir}/{PACKAGE}-{version}-*.whl")
    return wheels[-1]


def project_version() -> str:
    pyproject = tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))
    return pyproject["project"]["version"]


def find_python(python_spec: str) -> Path | None:
    result = subprocess.run(
        ["uv", "python", "find", python_spec],
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        return None
    return Path(result.stdout.strip())


def run(
    command: list[str],
    *,
    clean_python_env: bool = False,
    cwd: Path | None = None,
    env: dict[str, str] | None = None,
) -> subprocess.CompletedProcess[str]:
    command_env = os.environ.copy()
    if clean_python_env:
        for name in ("PYTHONHOME", "PYTHONPATH", "VIRTUAL_ENV"):
            command_env.pop(name, None)
    command_env.update(env or {})
    result = subprocess.run(
        command,
        check=False,
        capture_output=True,
        cwd=cwd,
        text=True,
        env=command_env,
    )
    if result.returncode != 0:
        output = "\n".join(part for part in (result.stdout, result.stderr) if part)
        raise SystemExit(f"command failed: {' '.join(command)}\n{output}")
    return result


if __name__ == "__main__":
    sys.exit(main())
