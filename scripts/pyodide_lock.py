from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
import tomllib
from collections.abc import Sequence
from pathlib import Path
from typing import Any

ROOT = Path(__file__).parents[1]
REQUIREMENTS_PATH = ROOT / ".github" / "pyodide" / "requirements.in"
LOCK_PATH = ROOT / ".github" / "pyodide" / "pylock.314.toml"
RUNTIME_PATH = ROOT / ".github" / "pyodide" / "runtime.json"
RUNTIME = json.loads(RUNTIME_PATH.read_text())
PYTHON_VERSION = RUNTIME["python-version"]
XBUILDENV_VERSION = RUNTIME["xbuildenv-version"]
POLARS_WHEEL_TAG = RUNTIME["polars-wheel-tag"]
HOST_WHEEL_TAGS = ("macosx_", "manylinux_", "musllinux_", "win_")


def _run(args: Sequence[str], *, env: dict[str, str] | None = None) -> None:
    subprocess.run(args, cwd=ROOT, env=env, check=True)  # noqa: S603


def _direct_requirements() -> dict[str, str]:
    requirements = {}
    for raw_line in REQUIREMENTS_PATH.read_text().splitlines():
        line = raw_line.partition("#")[0].strip()
        if line:
            name, separator, version = line.partition("==")
            if not separator or not version:
                raise ValueError(f"Pyodide runtime requirement must be exact: {line}")
            requirements[name.lower().replace("_", "-")] = version
    return requirements


def _packages(path: Path) -> dict[str, dict[str, Any]]:
    data = tomllib.loads(path.read_text())
    return {package["name"]: package for package in data["packages"]}


def validate_lock(path: Path) -> list[str]:
    packages = _packages(path)
    errors = []
    requirements = _direct_requirements()
    missing = requirements.keys() - packages.keys()
    if missing:
        errors.append(f"missing direct requirements: {', '.join(sorted(missing))}")
    for name, version in requirements.items():
        if name in packages and packages[name]["version"] != version:
            errors.append(f"{name} must resolve to {version}")

    workspace = tomllib.loads((ROOT / "pyproject.toml").read_text())
    build_requirement = (
        f"pyodide-build=={RUNTIME['pyodide-build-version']}; python_version >= '3.12'"
    )
    if workspace["dependency-groups"].get("pyodide-build") != [build_requirement]:
        errors.append(f"pyodide-build group must contain {build_requirement}")

    polars_wheels = packages.get("polars", {}).get("wheels", [])
    if len(polars_wheels) != 1 or POLARS_WHEEL_TAG not in polars_wheels[0]["name"]:
        errors.append(f"polars must resolve to one {POLARS_WHEEL_TAG} wheel")
    elif f"/pyodide/v{XBUILDENV_VERSION}/" not in polars_wheels[0]["url"]:
        errors.append(f"polars must come from Pyodide {XBUILDENV_VERSION}")

    for package in packages.values():
        for wheel in package.get("wheels", []):
            name = wheel["name"]
            if any(tag in name for tag in HOST_WHEEL_TAGS):
                errors.append(f"host-platform wheel in Pyodide lock: {name}")
            digest = wheel.get("hashes", {}).get("sha256", "")
            if len(digest) != 64:
                errors.append(f"missing SHA-256 hash for {name}")
    return errors


def _generate(output: Path) -> None:
    with tempfile.TemporaryDirectory(prefix="refkit-pyodide-lock-") as directory:
        work = Path(directory)
        host = work / "host"
        runtime = work / "runtime"
        _run(["uv", "venv", "--python", PYTHON_VERSION, str(host)])

        host_env = os.environ.copy()
        host_env["VIRTUAL_ENV"] = str(host)
        _run(
            [
                "uv",
                "sync",
                "--active",
                "--python",
                PYTHON_VERSION,
                "--locked",
                "--only-group",
                "pyodide-build",
            ],
            env=host_env,
        )

        pyodide_env = os.environ.copy()
        pyodide_env["PATH"] = f"{host / 'bin'}{os.pathsep}{pyodide_env['PATH']}"
        pyodide_env["PYODIDE_XBUILDENV_PATH"] = str(work / "xbuildenv")
        _run(
            [
                str(host / "bin" / "pyodide"),
                "xbuildenv",
                "install",
                XBUILDENV_VERSION,
            ],
            env=pyodide_env,
        )
        _run([str(host / "bin" / "pyodide"), "venv", str(runtime)], env=pyodide_env)
        _run(
            [
                str(runtime / "bin" / "python"),
                "-m",
                "pip",
                "lock",
                "-r",
                str(REQUIREMENTS_PATH),
                "-o",
                str(output),
            ],
            env=pyodide_env,
        )

    errors = validate_lock(output)
    if errors:
        raise SystemExit("Invalid Pyodide lock:\n" + "\n".join(f"- {error}" for error in errors))


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate the locked Pyodide test runtime.")
    parser.add_argument(
        "--check",
        action="store_true",
        help="Verify that the committed lock matches the runtime contract.",
    )
    return parser.parse_args()


def main() -> None:
    args = _parse_args()
    if args.check:
        errors = validate_lock(LOCK_PATH)
        if errors:
            raise SystemExit("Invalid Pyodide lock:\n" + "\n".join(f"- {e}" for e in errors))
        sys.stdout.write(f"Pyodide lock is current: {LOCK_PATH.relative_to(ROOT)}\n")
        return

    with tempfile.TemporaryDirectory(prefix="refkit-pyodide-output-") as directory:
        generated = Path(directory) / LOCK_PATH.name
        _generate(generated)
        generated.replace(LOCK_PATH)
        sys.stdout.write(f"Updated {LOCK_PATH.relative_to(ROOT)}\n")


if __name__ == "__main__":
    main()
