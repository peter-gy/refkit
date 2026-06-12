from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_PYTHONS = ("3.11", "3.12", "3.13", "3.14")


@dataclass(frozen=True)
class PackageSpec:
    distribution: str
    package_dir: str
    smoke_code: str

    @property
    def stem(self) -> str:
        return self.distribution.replace("-", "_")


SPECS = {
    "refkit": PackageSpec(
        distribution="refkit",
        package_dir="packages/refkit",
        smoke_code=r"""
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
    "package_file": str(refkit_file),
    "runtime": ".".join(map(str, sys.version_info[:3])),
    "version": expected,
}))
""",
    ),
    "polars-refkit": PackageSpec(
        distribution="polars-refkit",
        package_dir="packages/polars-refkit",
        smoke_code=r"""
import importlib.metadata as metadata
import json
import os
from pathlib import Path
import sys

import polars as pl
import polars_refkit
import polars_refkit._internal as native

expected = os.environ["REFKIT_EXPECTED_VERSION"]
prefix = Path(os.environ["REFKIT_EXPECTED_PREFIX"]).resolve()
versions = {
    "polars_refkit": polars_refkit.__version__,
    "native": native.__version__,
    "metadata": metadata.version("polars-refkit"),
}
if set(versions.values()) != {expected}:
    raise SystemExit(f"version mismatch: {versions}")

source = "@article{smoke, title={Smoke Title}, year={2024}}"
keys = pl.DataFrame({"bibtex": [source]}).select(
    polars_refkit.bibtex_keys("bibtex")
).to_series().to_list()[0]
if keys != ["smoke"]:
    raise SystemExit(f"unexpected plugin result: {keys!r}")

package_file = Path(polars_refkit.__file__).resolve()
native_file = Path(native.__file__).resolve()
for label, path in {"polars_refkit": package_file, "native": native_file}.items():
    if not path.is_relative_to(prefix):
        raise SystemExit(f"{label} imported from outside smoke venv: {path}")

print(json.dumps({
    "executable": sys.executable,
    "native_file": str(native_file),
    "package_file": str(package_file),
    "runtime": ".".join(map(str, sys.version_info[:3])),
    "version": expected,
}))
""",
    ),
}


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Import built refkit workspace wheels in fresh venvs."
    )
    parser.add_argument("--dist-dir", default="dist")
    parser.add_argument("--package", choices=[*sorted(SPECS), "all"], default="all")
    parser.add_argument("--wheel")
    parser.add_argument("--pythons", nargs="+", default=list(DEFAULT_PYTHONS))
    parser.add_argument(
        "--allow-missing",
        action="store_true",
        help="Report missing Python interpreters without failing.",
    )
    args = parser.parse_args()

    if args.package == "all" and args.wheel:
        raise SystemExit("--wheel requires --package")

    specs = SPECS.values() if args.package == "all" else [SPECS[args.package]]
    results = []
    with tempfile.TemporaryDirectory(prefix="refkit-release-smoke-") as temp:
        root = Path(temp)
        for spec in specs:
            version = project_version(spec)
            wheel = (
                Path(args.wheel) if args.wheel else find_wheel(Path(args.dist_dir), spec, version)
            )
            for python_spec in args.pythons:
                result = smoke_package(
                    spec=spec,
                    version=version,
                    wheel=wheel,
                    python_spec=python_spec,
                    root=root,
                )
                results.append(result)
                if result["status"] == "missing" and not args.allow_missing:
                    print(json.dumps(results, indent=2, sort_keys=True))
                    return 2

    print(json.dumps(results, indent=2, sort_keys=True))
    return 0


def smoke_package(
    *,
    spec: PackageSpec,
    version: str,
    wheel: Path,
    python_spec: str,
    root: Path,
) -> dict[str, object]:
    found = find_python(python_spec)
    if found is None:
        return {
            "package": spec.distribution,
            "python": python_spec,
            "status": "missing",
            "detail": f"uv python find {python_spec} failed",
        }

    venv = root / f"{spec.stem}-py{python_spec.replace('.', '')}"
    run(["uv", "venv", "--seed", "--python", str(found), str(venv)])
    venv_python = venv / "bin" / "python"
    run(["uv", "pip", "install", "--python", str(venv_python), str(wheel)])
    smoke = run(
        [str(venv_python), "-c", spec.smoke_code],
        cwd=root,
        clean_python_env=True,
        env={
            "REFKIT_EXPECTED_PREFIX": str(venv),
            "REFKIT_EXPECTED_VERSION": version,
        },
    )
    payload = json.loads(smoke.stdout)
    return {
        "package": spec.distribution,
        "python": python_spec,
        "status": "ok",
        **payload,
    }


def find_wheel(dist_dir: Path, spec: PackageSpec, version: str) -> Path:
    wheels = sorted(dist_dir.glob(f"{spec.stem}-{version}-*.whl"))
    if not wheels:
        raise SystemExit(f"missing wheel: {dist_dir}/{spec.stem}-{version}-*.whl")
    return wheels[-1]


def project_version(spec: PackageSpec) -> str:
    pyproject = tomllib.loads(
        (ROOT / spec.package_dir / "pyproject.toml").read_text(encoding="utf-8")
    )
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
