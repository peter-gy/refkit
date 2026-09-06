from __future__ import annotations

import argparse
import configparser
import glob
import json
import os
import subprocess
import sys
import tarfile
import zipfile
from collections.abc import Iterator
from pathlib import Path, PurePosixPath

GENERATED_SUFFIXES = (".pyc", ".pyo")
INTERNAL_DOCUMENTATION_DIRECTORY = "development_docs"
REFKIT_AGENT_PLUGIN_FILES = (
    "plugin.json",
    "skills/refkit/SKILL.md",
    "skills/refkit/agents/openai.yaml",
    "skills/refkit/references/contracts.md",
    "skills/refkit/references/workflows.md",
)
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


def agent_plugin_violations(path: Path) -> list[str]:
    """Return RefKit Agent Plugin archive contract violations."""

    if not path.name.startswith("refkit-"):
        return []
    contents = dict(_member_contents(path))
    if path.suffix == ".whl":
        return _wheel_agent_plugin_violations(contents)
    return _sdist_agent_plugin_violations(contents)


def _wheel_agent_plugin_violations(contents: dict[str, bytes]) -> list[str]:
    violations = []
    dist_info_candidates = [
        name.removesuffix("/WHEEL") for name in contents if name.endswith(".dist-info/WHEEL")
    ]
    if len(dist_info_candidates) != 1:
        return ["wheel must contain exactly one .dist-info/WHEEL file"]

    dist_info = dist_info_candidates[0]
    plugin_root = f"{dist_info.removesuffix('.dist-info')}.agent-plugin"
    expected_payload = {f"{plugin_root}/{relative}" for relative in REFKIT_AGENT_PLUGIN_FILES}
    actual_payload = {name for name in contents if name.startswith(f"{plugin_root}/")}
    if actual_payload != expected_payload:
        violations.append(
            "wheel Agent Plugin payload mismatch: "
            f"missing={sorted(expected_payload - actual_payload)}, "
            f"extra={sorted(actual_payload - expected_payload)}"
        )

    marker_path = f"{dist_info}/agent_plugins.json"
    marker = contents.get(marker_path)
    if marker is None:
        violations.append(f"wheel is missing {marker_path}")
    else:
        try:
            marker_value = json.loads(marker)
        except (UnicodeDecodeError, json.JSONDecodeError):
            violations.append(f"wheel contains invalid {marker_path}")
        else:
            expected_marker = {
                "root": plugin_root,
                "files": list(REFKIT_AGENT_PLUGIN_FILES),
            }
            if marker_value != expected_marker:
                violations.append(f"wheel contains unexpected {marker_path}")

    if "refkit/agent.py" not in contents:
        violations.append("wheel is missing refkit/agent.py")

    entry_points_path = f"{dist_info}/entry_points.txt"
    entry_points = contents.get(entry_points_path)
    if entry_points is None:
        violations.append(f"wheel is missing {entry_points_path}")
    else:
        parser = configparser.ConfigParser()
        try:
            parser.read_string(entry_points.decode())
            capability = parser.get("marimo.agent.capability", "refkit")
        except (UnicodeDecodeError, configparser.Error, KeyError):
            capability = None
        if capability != "refkit.agent":
            violations.append("wheel must register refkit = refkit.agent")

    metadata_path = f"{dist_info}/METADATA"
    metadata = contents.get(metadata_path, b"").decode(errors="replace")
    if "Requires-Dist: agent-plugins==0.2.0" not in metadata:
        violations.append("wheel must require agent-plugins==0.2.0")
    return violations


def _sdist_agent_plugin_violations(contents: dict[str, bytes]) -> list[str]:
    roots = {PurePosixPath(name).parts[0] for name in contents if PurePosixPath(name).parts}
    if len(roots) != 1:
        return ["sdist must contain exactly one archive root"]
    root = roots.pop()
    plugin_root = f"{root}/.agent-plugin"
    expected_payload = {f"{plugin_root}/{relative}" for relative in REFKIT_AGENT_PLUGIN_FILES}
    actual_payload = {name for name in contents if name.startswith(f"{plugin_root}/")}
    violations = []
    if actual_payload != expected_payload:
        violations.append(
            "sdist Agent Plugin payload mismatch: "
            f"missing={sorted(expected_payload - actual_payload)}, "
            f"extra={sorted(actual_payload - expected_payload)}"
        )
    for relative in ("build_backend.py", "src/refkit/agent.py"):
        member = f"{root}/{relative}"
        if member not in contents:
            violations.append(f"sdist is missing {member}")
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
        errors.extend(
            f"{distribution}: {violation}" for violation in agent_plugin_violations(distribution)
        )
    if errors:
        sys.stderr.write("Distribution contract failed:\n" + "\n".join(errors) + "\n")
        return 1
    sys.stdout.write("Distribution contract passed\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
