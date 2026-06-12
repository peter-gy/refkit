from __future__ import annotations

import argparse
import json
import sys
import tarfile
import tomllib
import zipfile
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LOCAL_ONLY_GOAL_PART = "no" + "git"
NATIVE_EXTENSION_SUFFIXES = {".pyd", ".so"}
LEGACY_LICENSE_NAME = "MI" + "T"
LEGACY_LICENSE_FILE = "LICENSE-" + LEGACY_LICENSE_NAME
LEGACY_DUAL_LICENSE = f"{LEGACY_LICENSE_NAME} OR Apache-2.0".encode()
FORBIDDEN_PARTS = {
    ".DS_Store",
    ".pytest_cache",
    ".ruff_cache",
    ".venv",
    "Thumbs.db",
    "__pycache__",
    "benchmark",
    "dist",
    LEGACY_LICENSE_FILE,
    LOCAL_ONLY_GOAL_PART,
    "target",
}


@dataclass(frozen=True)
class PackageSpec:
    distribution: str
    package_dir: str
    import_package: str
    native_prefix: str
    requires_dist: Mapping[str, frozenset[str]]
    sdist_required: frozenset[str]
    wheel_required: frozenset[str]

    @property
    def stem(self) -> str:
        return self.distribution.replace("-", "_")


SPECS = {
    "refkit": PackageSpec(
        distribution="refkit",
        package_dir="packages/refkit",
        import_package="refkit",
        native_prefix="_native",
        requires_dist={},
        sdist_required=frozenset(
            {
                "Cargo.lock",
                "Cargo.toml",
                "LICENSE-APACHE",
                "README.md",
                "crates/refkit-core/Cargo.toml",
                "crates/refkit-core/src/lib.rs",
                "crates/refkit-core/src/library.rs",
                "crates/refkit-core/src/render.rs",
                "crates/refkit-core/src/style_analysis.rs",
                "crates/refkit-core/src/strings.rs",
                "packages/refkit/Cargo.toml",
                "packages/refkit/src/lib.rs",
                "packages/refkit/src/raw.rs",
                "packages/refkit/src/rendered.rs",
                "pyproject.toml",
                "src/refkit/__init__.py",
                "src/refkit/__init__.pyi",
                "src/refkit/_native.pyi",
                "src/refkit/py.typed",
            }
        ),
        wheel_required=frozenset(
            {
                "refkit/__init__.py",
                "refkit/__init__.pyi",
                "refkit/_native.pyi",
                "refkit/py.typed",
            }
        ),
    ),
    "polars-refkit": PackageSpec(
        distribution="polars-refkit",
        package_dir="packages/polars-refkit",
        import_package="polars_refkit",
        native_prefix="_internal",
        requires_dist={"polars": frozenset({">=1.41", "<1.42"})},
        sdist_required=frozenset(
            {
                "Cargo.lock",
                "Cargo.toml",
                "LICENSE-APACHE",
                "README.md",
                "crates/refkit-core/Cargo.toml",
                "crates/refkit-core/src/lib.rs",
                "crates/refkit-core/src/library.rs",
                "crates/refkit-core/src/render.rs",
                "crates/refkit-core/src/style_analysis.rs",
                "crates/refkit-core/src/strings.rs",
                "packages/polars-refkit/Cargo.toml",
                "packages/polars-refkit/src/expressions.rs",
                "packages/polars-refkit/src/lib.rs",
                "polars_refkit/__init__.py",
                "polars_refkit/__init__.pyi",
                "polars_refkit/_internal.pyi",
                "polars_refkit/py.typed",
                "pyproject.toml",
            }
        ),
        wheel_required=frozenset(
            {
                "polars_refkit/__init__.py",
                "polars_refkit/__init__.pyi",
                "polars_refkit/_internal.pyi",
                "polars_refkit/py.typed",
            }
        ),
    ),
}


def main() -> int:
    parser = argparse.ArgumentParser(description="Inspect refkit workspace package artifacts.")
    parser.add_argument("--package", choices=[*sorted(SPECS), "all"], default="all")
    parser.add_argument("--sdist", type=Path)
    parser.add_argument("--wheel", type=Path)
    parser.add_argument("dist_dir", nargs="?", default="dist")
    args = parser.parse_args()

    if args.package == "all" and (args.sdist or args.wheel):
        raise SystemExit("--sdist and --wheel require --package")

    dist_dir = Path(args.dist_dir)
    specs = SPECS.values() if args.package == "all" else [SPECS[args.package]]
    payload = [inspect_package(spec, dist_dir, args.sdist, args.wheel) for spec in specs]
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


def inspect_package(
    spec: PackageSpec,
    dist_dir: Path,
    sdist_arg: Path | None,
    wheel_arg: Path | None,
) -> dict[str, object]:
    version = project_version(spec)
    sdist = sdist_arg or dist_dir / f"{spec.stem}-{version}.tar.gz"
    wheels = [wheel_arg] if wheel_arg else sorted(dist_dir.glob(f"{spec.stem}-{version}-*.whl"))
    if not sdist.exists():
        raise SystemExit(f"missing sdist: {sdist}")
    if not wheels:
        raise SystemExit(f"missing wheel: {dist_dir}/{spec.stem}-{version}-*.whl")

    wheel = wheels[-1]
    sdist_prefix = f"{spec.stem}-{version}/"
    dist_info = f"{spec.stem}-{version}.dist-info"
    sdist_names = read_sdist_names(sdist)
    wheel_names = read_wheel_names(wheel)

    assert_no_forbidden_paths(sdist_names, "sdist")
    assert_no_forbidden_paths(wheel_names, "wheel")
    assert_no_forbidden_payloads_in_sdist(sdist)
    assert_no_forbidden_payloads_in_wheel(wheel)
    assert_required(
        sdist_names,
        "sdist",
        {sdist_prefix + name for name in spec.sdist_required},
    )
    assert_required(
        wheel_names,
        "wheel",
        {
            *spec.wheel_required,
            f"{dist_info}/METADATA",
            f"{dist_info}/WHEEL",
            f"{dist_info}/licenses/LICENSE-APACHE",
        },
    )
    if not any(is_native_extension(spec, name) for name in wheel_names):
        raise SystemExit(f"{spec.distribution} wheel is missing native extension")

    metadata = read_wheel_text(wheel, f"{dist_info}/METADATA")
    assert_metadata_line(metadata, f"Name: {spec.distribution}")
    assert_metadata_line(metadata, f"Version: {version}")
    assert_apache_license_metadata(metadata)
    assert_metadata_field(metadata, "Requires-Python", {">=3.11", "<3.15"})
    for requirement, expected_parts in spec.requires_dist.items():
        assert_metadata_requirement(metadata, requirement, expected_parts)
    if "](docs/" in metadata:
        raise SystemExit("wheel metadata contains a relative docs/ link")

    return {
        "package": spec.distribution,
        "sdist": str(sdist),
        "sdist_files": len(sdist_names),
        "status": "ok",
        "version": version,
        "wheel": str(wheel),
        "wheel_files": len(wheel_names),
    }


def read_sdist_names(path: Path) -> set[str]:
    with tarfile.open(path) as archive:
        return set(archive.getnames())


def project_version(spec: PackageSpec) -> str:
    pyproject = tomllib.loads(
        (ROOT / spec.package_dir / "pyproject.toml").read_text(encoding="utf-8")
    )
    return pyproject["project"]["version"]


def is_native_extension(spec: PackageSpec, name: str) -> bool:
    path = Path(name)
    return (
        path.parent == Path(spec.import_package)
        and path.name.startswith(f"{spec.native_prefix}.")
        and path.suffix in NATIVE_EXTENSION_SUFFIXES
    )


def read_wheel_names(path: Path) -> set[str]:
    with zipfile.ZipFile(path) as archive:
        return set(archive.namelist())


def read_wheel_text(path: Path, name: str) -> str:
    with zipfile.ZipFile(path) as archive:
        return archive.read(name).decode("utf-8")


def assert_no_forbidden_paths(names: set[str], artifact: str) -> None:
    for name in names:
        if LOCAL_ONLY_GOAL_PART in name:
            raise SystemExit(f"{artifact} contains forbidden path {name!r}")
        parts = set(Path(name).parts)
        if forbidden := sorted(parts & FORBIDDEN_PARTS):
            raise SystemExit(f"{artifact} contains forbidden path {name!r}: {forbidden}")


def assert_no_forbidden_payloads_in_sdist(path: Path) -> None:
    with tarfile.open(path) as archive:
        for member in archive.getmembers():
            if not member.isfile():
                continue
            extracted = archive.extractfile(member)
            if extracted is None:
                continue
            assert_no_forbidden_payload(extracted.read(), "sdist", member.name)


def assert_no_forbidden_payloads_in_wheel(path: Path) -> None:
    with zipfile.ZipFile(path) as archive:
        for name in archive.namelist():
            if name.endswith("/"):
                continue
            scan_license_text = should_scan_license_text_in_wheel(name)
            assert_no_forbidden_payload(
                archive.read(name),
                "wheel",
                name,
                scan_license_text=scan_license_text,
            )


def should_scan_license_text_in_wheel(name: str) -> bool:
    path = Path(name)
    if path.suffix in NATIVE_EXTENSION_SUFFIXES:
        return False
    return "sboms" not in path.parts


def assert_no_forbidden_payload(
    payload: bytes,
    artifact: str,
    name: str,
    *,
    scan_license_text: bool = True,
) -> None:
    if LOCAL_ONLY_GOAL_PART.encode() in payload:
        raise SystemExit(f"{artifact} file {name!r} contains a forbidden local helper token")
    if not scan_license_text:
        return
    for forbidden in (LEGACY_DUAL_LICENSE, LEGACY_LICENSE_FILE.encode()):
        if forbidden in payload:
            raise SystemExit(f"{artifact} file {name!r} contains forbidden license text")


def assert_required(names: set[str], artifact: str, required: set[str]) -> None:
    missing = sorted(required - names)
    if missing:
        raise SystemExit(f"{artifact} is missing required files: {missing}")


def assert_metadata_line(metadata: str, expected: str) -> None:
    if expected not in metadata.splitlines():
        raise SystemExit(f"wheel metadata is missing {expected!r}")


def assert_metadata_field(metadata: str, field: str, expected_parts: set[str]) -> None:
    prefix = f"{field}: "
    values = [
        line.removeprefix(prefix) for line in metadata.splitlines() if line.startswith(prefix)
    ]
    if not values:
        raise SystemExit(f"wheel metadata is missing {field!r}")
    actual_parts = {part.strip() for part in values[0].split(",")}
    if actual_parts != expected_parts:
        raise SystemExit(
            f"wheel metadata field {field!r} is {values[0]!r}, expected {sorted(expected_parts)}"
        )


def assert_apache_license_metadata(metadata: str) -> None:
    lines = set(metadata.splitlines())
    if lines.isdisjoint({"License: Apache-2.0", "License-Expression: Apache-2.0"}):
        raise SystemExit("wheel metadata is missing Apache-2.0 license metadata")
    for line in lines:
        if LEGACY_LICENSE_NAME in line:
            raise SystemExit(f"wheel metadata contains forbidden legacy license text: {line!r}")


def assert_metadata_requirement(
    metadata: str,
    requirement: str,
    expected_parts: frozenset[str],
) -> None:
    prefix = f"Requires-Dist: {requirement}"
    values = [line for line in metadata.splitlines() if line.startswith(prefix)]
    if not values:
        raise SystemExit(f"wheel metadata is missing dependency {requirement!r}")
    actual = values[0].removeprefix(prefix).strip()
    actual = actual.removeprefix("(").removesuffix(")")
    actual_parts = {part.strip() for part in actual.split(",") if part.strip()}
    if actual_parts != set(expected_parts):
        raise SystemExit(
            f"wheel dependency {requirement!r} is {actual!r}, expected {sorted(expected_parts)}"
        )


if __name__ == "__main__":
    sys.exit(main())
