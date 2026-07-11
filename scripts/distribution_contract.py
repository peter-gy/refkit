from __future__ import annotations

import argparse
import glob
import os
import subprocess
import sys
import tarfile
import zipfile
from collections.abc import Iterator
from pathlib import Path, PurePosixPath

GENERATED_SUFFIXES = (".pyc", ".pyo")
INTERNAL_DOCUMENTATION_DIRECTORY = "development_docs"
ROOT = Path(__file__).resolve().parents[1]
SBOM_LOCAL_REFERENCE_MARKERS = (b"path+file://", b"download_url=file://")
KNOWN_CI_BUILD_PATHS = (
    "/home/runner/",
    "/Users/runner/",
    "C:\\Users\\runneradmin\\",
    "C:/Users/runneradmin/",
    "D:\\a\\",
    "D:/a/",
    "/github/home/",
    "/opt/homebrew/Cellar/rust/",
    "/root/.cargo/",
    "/root/.rustup/",
    "/usr/local/rustup/",
)


def _members(path: Path) -> list[str]:
    if path.suffix == ".whl":
        with zipfile.ZipFile(path) as archive:
            return archive.namelist()
    if path.name.endswith(".tar.gz"):
        with tarfile.open(path, "r:gz") as archive:
            return archive.getnames()
    raise ValueError(f"unsupported distribution: {path}")


def generated_members(path: Path) -> list[str]:
    return [
        member
        for member in _members(path)
        if "__pycache__" in Path(member).parts or member.endswith(GENERATED_SUFFIXES)
    ]


def internal_document_members(path: Path) -> list[str]:
    return [
        member
        for member in _members(path)
        if INTERNAL_DOCUMENTATION_DIRECTORY in PurePosixPath(member).parts
    ]


def _member_contents(path: Path) -> Iterator[tuple[str, bytes]]:
    if path.suffix == ".whl":
        with zipfile.ZipFile(path) as archive:
            for member in archive.namelist():
                yield member, archive.read(member)
        return
    if path.name.endswith(".tar.gz"):
        with tarfile.open(path, "r:gz") as archive:
            for member in archive.getmembers():
                if not member.isfile():
                    continue
                source = archive.extractfile(member)
                if source is not None:
                    yield member.name, source.read()
        return
    raise ValueError(f"unsupported distribution: {path}")


def _builder_path_markers() -> tuple[bytes, ...]:
    try:
        rustc = subprocess.run(
            ["rustc", "--print", "sysroot"],
            check=False,
            capture_output=True,
            text=True,
        )
        rust_sysroot = rustc.stdout.strip()
    except OSError:
        rust_sysroot = ""
    paths = {
        str(ROOT),
        str(Path.home()),
        rust_sysroot,
        *KNOWN_CI_BUILD_PATHS,
        *(
            value
            for name in (
                "GITHUB_WORKSPACE",
                "CARGO_HOME",
                "RUNNER_TEMP",
                "RUNNER_TOOL_CACHE",
                "RUNNER_WORKSPACE",
                "RUSTUP_HOME",
                "USERPROFILE",
            )
            if (value := os.environ.get(name))
        ),
    }
    variants = {
        variant.encode()
        for path in paths
        for variant in (path, path.replace("\\", "/"), path.replace("/", "\\"))
        if len(variant) > 3
    }
    return tuple(sorted(variants, key=len, reverse=True))


def content_violations(path: Path) -> list[str]:
    violations = []
    builder_paths = _builder_path_markers()
    for member, content in _member_contents(path):
        if ".dist-info/sboms/" in member and any(
            marker in content for marker in SBOM_LOCAL_REFERENCE_MARKERS
        ):
            violations.append(f"{member}: contains a local SBOM reference")
        if any(marker in content for marker in builder_paths):
            violations.append(f"{member}: embeds a builder path")
    return violations


def distribution_paths(arguments: list[str]) -> tuple[list[Path], list[str]]:
    paths = []
    unmatched = []
    for argument in arguments:
        matches = glob.glob(argument) if glob.has_magic(argument) else [argument]
        files = sorted(Path(match) for match in matches if Path(match).is_file())
        if files:
            paths.extend(files)
        else:
            unmatched.append(argument)
    return paths, unmatched


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Validate RefKit distribution archive contents.")
    parser.add_argument("distributions", nargs="+")
    return parser.parse_args()


def main() -> int:
    distributions, unmatched = distribution_paths(_parse_args().distributions)
    errors = [f"no distribution files matched {argument}" for argument in unmatched]
    for distribution in distributions:
        errors.extend(
            f"{distribution}: generated member {member}"
            for member in generated_members(distribution)
        )
        errors.extend(
            f"{distribution}: internal documentation member {member}"
            for member in internal_document_members(distribution)
        )
        errors.extend(
            f"{distribution}: {violation}" for violation in content_violations(distribution)
        )
    if errors:
        sys.stderr.write("Distribution contract failed:\n" + "\n".join(errors) + "\n")
        return 1
    sys.stdout.write("Distribution contract passed\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
