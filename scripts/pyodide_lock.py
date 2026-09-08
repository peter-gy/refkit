from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from collections.abc import Sequence
from pathlib import Path
from typing import Any
from urllib.parse import urlsplit

if sys.version_info >= (3, 11):
    import tomllib
else:
    import tomli as tomllib

ROOT = Path(__file__).parents[1]
REQUIREMENTS_PATH = ROOT / ".github" / "pyodide" / "requirements.in"
LOCK_PATH = ROOT / ".github" / "pyodide" / "pylock.314.toml"
RUNTIME_PATH = ROOT / ".github" / "pyodide" / "runtime.json"
RUNTIME = json.loads(RUNTIME_PATH.read_text())
PYTHON_VERSION = RUNTIME["python-version"]
XBUILDENV_VERSION = RUNTIME["xbuildenv-version"]
POLARS_WHEEL_TAG = RUNTIME["polars-wheel-tag"]
POLARS_PLUGIN_ABI = RUNTIME["polars-plugin-abi"]
POLARS_CARGO_LOCK_PATH = ROOT / "packages" / "polars-refkit" / "rust" / "Cargo.lock"
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
    if data.get("lock-version") != "1.0" or not isinstance(data.get("packages"), list):
        raise ValueError("lock must declare lock-version 1.0 and a packages array")
    packages = {}
    for package in data["packages"]:
        if not isinstance(package, dict):
            raise ValueError("each package must be a table")
        name = package.get("name")
        if not isinstance(name, str) or re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", name) is None:
            raise ValueError("package name must be a normalized distribution name")
        if name in packages:
            raise ValueError(f"duplicate package: {name}")
        if not isinstance(package.get("version"), str) or not package["version"]:
            raise ValueError(f"{name} must declare a version")
        wheels = package.get("wheels")
        if not isinstance(wheels, list) or len(wheels) != 1 or not isinstance(wheels[0], dict):
            raise ValueError(f"{name} must resolve to exactly one wheel")
        wheel = wheels[0]
        if not isinstance(wheel.get("name"), str) or not isinstance(wheel.get("url"), str):
            raise ValueError(f"{name} wheel must declare a name and URL")
        if not isinstance(wheel.get("hashes"), dict):
            raise ValueError(f"{name} wheel must declare hashes")
        if "sdist" in package:
            raise ValueError(f"{name} must resolve to a wheel")
        packages[name] = package
    return packages


def _cargo_versions(path: Path) -> dict[str, set[str]]:
    packages = tomllib.loads(path.read_text())["package"]
    names = ("polars", "polars-core", "pyo3", "pyo3-polars")
    return {
        name: {package["version"] for package in packages if package["name"] == name}
        for name in names
    }


def validate_lock(path: Path) -> list[str]:
    try:
        packages = _packages(path)
    except (OSError, ValueError, tomllib.TOMLDecodeError) as error:
        return [str(error)]
    errors = []
    requirements = _direct_requirements()
    missing = requirements.keys() - packages.keys()
    if missing:
        errors.append(f"missing direct requirements: {', '.join(sorted(missing))}")
    for name, version in requirements.items():
        if name in packages and packages[name]["version"] != version:
            errors.append(f"{name} must resolve to {version}")

    expected_python_polars = POLARS_PLUGIN_ABI["python-polars"]
    if requirements.get("polars") != expected_python_polars:
        errors.append(
            f"Pyodide Polars requirement must be {expected_python_polars} for the plugin ABI"
        )

    cargo_versions = _cargo_versions(POLARS_CARGO_LOCK_PATH)
    cargo_contract = {
        "polars": POLARS_PLUGIN_ABI["rust-polars"],
        "polars-core": POLARS_PLUGIN_ABI["rust-polars"],
        "pyo3": POLARS_PLUGIN_ABI["pyo3"],
        "pyo3-polars": POLARS_PLUGIN_ABI["pyo3-polars"],
    }
    for name, version in cargo_contract.items():
        if cargo_versions[name] != {version}:
            errors.append(f"{name} must resolve to {version} for the Polars plugin ABI")

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
            if not isinstance(digest, str) or re.fullmatch(r"[0-9a-f]{64}", digest) is None:
                errors.append(f"invalid SHA-256 hash for {name}")
            try:
                url = urlsplit(wheel["url"])
            except ValueError:
                errors.append(f"invalid wheel URL for {name}")
                continue
            if url.scheme != "https" or url.netloc not in {
                "files.pythonhosted.org",
                "cdn.jsdelivr.net",
            }:
                errors.append(f"unsupported wheel source for {name}")
            if url.query or url.fragment or not url.path.endswith(f"/{name}"):
                errors.append(f"wheel URL must identify {name}")
            if url.netloc == "cdn.jsdelivr.net" and not url.path.startswith(
                f"/pyodide/v{XBUILDENV_VERSION}/full/"
            ):
                errors.append(f"{name} must come from Pyodide {XBUILDENV_VERSION}")
            expected_prefix = f"{package['name'].replace('-', '_')}-{package['version']}-"
            if not name.startswith(expected_prefix) or not name.endswith(
                ("-py3-none-any.whl", f"-{POLARS_WHEEL_TAG}.whl")
            ):
                errors.append(f"incompatible wheel identity or runtime tag: {name}")
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
        sys.stdout.write(
            f"Pyodide lock matches the runtime contract: {LOCK_PATH.relative_to(ROOT)}\n"
        )
        return

    with tempfile.TemporaryDirectory(prefix="refkit-pyodide-output-") as directory:
        generated = Path(directory) / LOCK_PATH.name
        _generate(generated)
        generated.replace(LOCK_PATH)
        sys.stdout.write(f"Updated {LOCK_PATH.relative_to(ROOT)}\n")


if __name__ == "__main__":
    main()
