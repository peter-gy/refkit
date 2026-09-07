from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def _archives(directory: Path) -> list[Path]:
    return sorted([*directory.glob("*.whl"), *directory.glob("*.tar.gz")])


def _version(command: list[str]) -> str:
    return subprocess.run(command, check=True, capture_output=True, text=True).stdout.strip()


def record(directory: Path, package: str, artifact: str, rust_toolchain: str | None = None) -> Path:
    archives = _archives(directory)
    if len(archives) != 1:
        raise ValueError(f"{artifact} must contain exactly one distribution archive")
    if archives[0].suffix == ".whl" and rust_toolchain is None:
        raise ValueError("wheel provenance requires --rust-toolchain used by Maturin")
    constraints = tomllib.loads((ROOT / "pyproject.toml").read_text())["tool"]["uv"][
        "build-constraint-dependencies"
    ]
    document = {
        "schema": 1,
        "package": package,
        "artifact": artifact,
        "source": os.environ.get("GITHUB_SHA") or _version(["git", "rev-parse", "HEAD"]),
        "tools": {
            "python": platform.python_version(),
            "rustc": (
                _version(["rustup", "run", rust_toolchain, "rustc", "--version"])
                if rust_toolchain is not None
                else None
            ),
            "rust_toolchain": rust_toolchain,
            "uv": _version(["uv", "--version"]),
            "build_constraints": constraints,
        },
        "files": {path.name: hashlib.sha256(path.read_bytes()).hexdigest() for path in archives},
    }
    destination = directory / f"{artifact}.json"
    destination.write_text(json.dumps(document, indent=2) + "\n")
    return destination


def check(directory: Path, package: str, version: str, source: str) -> list[str]:
    prefix = package.replace("-", "_")
    runtime = json.loads((ROOT / ".github/pyodide/runtime.json").read_text())
    pyemscripten = f"pyemscripten_{runtime['python-version']}"
    expected = {
        f"{prefix}_{suffix}"
        for suffix in (
            "cpython_Linux",
            "cpython_macOS",
            "cpython_Windows",
            pyemscripten,
            "sdist",
        )
    }
    errors = []
    constraints = tomllib.loads((ROOT / "pyproject.toml").read_text())["tool"]["uv"][
        "build-constraint-dependencies"
    ]
    files: dict[str, str] = {}
    manifests = list(directory.glob("*.json"))
    if {path.stem for path in manifests} != expected:
        errors.append(
            "artifact manifests must cover Linux, macOS, Windows, PyEmscripten, and sdist"
        )
    for path in manifests:
        try:
            document = json.loads(path.read_text())
            if (
                document["schema"] != 1
                or document["package"] != package
                or document["artifact"] != path.stem
                or document["source"] != source
            ):
                errors.append(f"{path.name}: package, artifact, or source identity mismatch")
            tools = document["tools"]
            if not isinstance(tools, dict) or any(
                not isinstance(tools.get(name), str) or not tools[name] for name in ("python", "uv")
            ):
                raise ValueError("expected Python and uv build versions")
            if path.stem.endswith("_sdist"):
                if tools.get("rustc") is not None or tools.get("rust_toolchain") is not None:
                    raise ValueError("sdist provenance must describe source packaging")
            elif any(
                not isinstance(tools.get(name), str) or not tools[name]
                for name in ("rustc", "rust_toolchain")
            ):
                raise ValueError("wheel provenance must identify the selected Rust compiler")
            if tools.get("build_constraints") != constraints:
                errors.append(f"{path.name}: build constraints differ from release source")
            recorded = document["files"]
            if not isinstance(recorded, dict) or len(recorded) != 1:
                raise ValueError("expected exactly one archive")
            for name, digest in recorded.items():
                if (
                    not isinstance(name, str)
                    or Path(name).name != name
                    or not isinstance(digest, str)
                    or re.fullmatch(r"[0-9a-f]{64}", digest) is None
                ):
                    raise ValueError("invalid archive name or SHA-256")
                if name in files:
                    errors.append(f"{name}: appears in multiple artifacts")
                target = path.stem.removeprefix(f"{prefix}_")
                tags = {
                    "cpython_Linux": ("manylinux_", "musllinux_", "linux_"),
                    "cpython_macOS": ("macosx_",),
                    "cpython_Windows": ("win_",),
                    pyemscripten: ("pyemscripten_",),
                    "sdist": (".tar.gz",),
                }
                if not any(tag in name for tag in tags.get(target, ())):
                    errors.append(f"{name}: does not match artifact target {target}")
                files[name] = digest
        except (ValueError, KeyError, TypeError) as error:
            errors.append(f"{path.name}: invalid artifact manifest: {error}")
    archives = _archives(directory)
    if {path.name for path in archives} != files.keys():
        errors.append("distribution archives do not match the recorded artifact set")
    normalized = version.replace("-rc.", "rc")
    for archive in archives:
        if (
            not archive.name.startswith(f"{prefix}-{normalized}-")
            and archive.name != f"{prefix}-{normalized}.tar.gz"
        ):
            errors.append(f"{archive.name}: expected {package} {normalized}")
        if hashlib.sha256(archive.read_bytes()).hexdigest() != files.get(archive.name):
            errors.append(f"{archive.name}: SHA-256 mismatch")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description="Record and verify package artifact provenance.")
    subparsers = parser.add_subparsers(dest="command", required=True)
    for name in ("record", "check"):
        command = subparsers.add_parser(name)
        command.add_argument("directory", type=Path)
        command.add_argument("--package", choices=("refkit", "polars-refkit"), required=True)
        if name == "record":
            command.add_argument("--artifact", required=True)
            command.add_argument("--rust-toolchain", help="Rust toolchain selected by Maturin")
        else:
            command.add_argument("--version", required=True)
    args = parser.parse_args()
    try:
        if args.command == "record":
            record(args.directory, args.package, args.artifact, args.rust_toolchain)
        else:
            source = os.environ.get("GITHUB_SHA") or _version(["git", "rev-parse", "HEAD"])
            errors = check(args.directory, args.package, args.version, source)
            if errors:
                raise ValueError("\n".join(errors))
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        sys.stderr.write(f"Artifact manifest failed:\n{error}\n")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
