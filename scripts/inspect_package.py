from __future__ import annotations

import argparse
import json
import sys
import tarfile
import tomllib
import zipfile
from pathlib import Path

PACKAGE = "refkit"
ROOT = Path(__file__).resolve().parents[1]
LOCAL_ONLY_GOAL_PART = "no" + "git"
NATIVE_EXTENSION_SUFFIXES = {".pyd", ".so"}
FORBIDDEN_PARTS = {
    ".pytest_cache",
    ".ruff_cache",
    ".venv",
    "__pycache__",
    "benchmark",
    "dist",
    LOCAL_ONLY_GOAL_PART,
    "target",
}


def main() -> int:
    parser = argparse.ArgumentParser(description="Inspect refkit package artifacts.")
    parser.add_argument("--sdist", type=Path)
    parser.add_argument("--wheel", type=Path)
    parser.add_argument("dist_dir", nargs="?", default="dist")
    args = parser.parse_args()

    version = project_version()
    dist_dir = Path(args.dist_dir)
    sdist = args.sdist or dist_dir / f"{PACKAGE}-{version}.tar.gz"
    wheels = [args.wheel] if args.wheel else sorted(dist_dir.glob(f"{PACKAGE}-{version}-*.whl"))
    if not sdist.exists():
        raise SystemExit(f"missing sdist: {sdist}")
    if not wheels:
        raise SystemExit(f"missing wheel: {dist_dir}/{PACKAGE}-{version}-*.whl")

    sdist_names = read_sdist_names(sdist)
    wheel_names = read_wheel_names(wheels[-1])
    assert_no_forbidden_paths(sdist_names, "sdist")
    assert_no_forbidden_paths(wheel_names, "wheel")
    assert_no_forbidden_payloads_in_sdist(sdist)
    assert_no_forbidden_payloads_in_wheel(wheels[-1])
    assert_required(
        sdist_names,
        "sdist",
        {
            f"{PACKAGE}-{version}/Cargo.lock",
            f"{PACKAGE}-{version}/Cargo.toml",
            f"{PACKAGE}-{version}/LICENSE-APACHE",
            f"{PACKAGE}-{version}/LICENSE-MIT",
            f"{PACKAGE}-{version}/README.md",
            f"{PACKAGE}-{version}/pyproject.toml",
            f"{PACKAGE}-{version}/src/lib.rs",
            f"{PACKAGE}-{version}/src/raw.rs",
            f"{PACKAGE}-{version}/src/refkit/__init__.py",
            f"{PACKAGE}-{version}/src/refkit/__init__.pyi",
            f"{PACKAGE}-{version}/src/refkit/_native.pyi",
            f"{PACKAGE}-{version}/src/refkit/py.typed",
        },
    )
    assert_required(
        wheel_names,
        "wheel",
        {
            "refkit/__init__.py",
            "refkit/__init__.pyi",
            "refkit/_native.pyi",
            "refkit/py.typed",
            f"{PACKAGE}-{version}.dist-info/METADATA",
            f"{PACKAGE}-{version}.dist-info/WHEEL",
            f"{PACKAGE}-{version}.dist-info/licenses/LICENSE-APACHE",
            f"{PACKAGE}-{version}.dist-info/licenses/LICENSE-MIT",
        },
    )
    if not any(is_native_extension(name) for name in wheel_names):
        raise SystemExit("wheel is missing native extension")

    metadata = read_wheel_text(wheels[-1], f"{PACKAGE}-{version}.dist-info/METADATA")
    assert_metadata_line(metadata, f"Version: {version}")
    assert_metadata_field(metadata, "Requires-Python", {">=3.11", "<3.15"})
    if "](docs/" in metadata:
        raise SystemExit("wheel metadata contains a relative docs/ link")

    payload = {
        "sdist": str(sdist),
        "sdist_files": len(sdist_names),
        "wheel": str(wheels[-1]),
        "wheel_files": len(wheel_names),
        "status": "ok",
    }
    print(json.dumps(payload, indent=2, sort_keys=True))
    return 0


def read_sdist_names(path: Path) -> set[str]:
    with tarfile.open(path) as archive:
        return set(archive.getnames())


def project_version() -> str:
    pyproject = tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))
    return pyproject["project"]["version"]


def is_native_extension(name: str) -> bool:
    path = Path(name)
    return (
        path.parent == Path("refkit")
        and path.name.startswith("_native.")
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
            assert_no_forbidden_payload(archive.read(name), "wheel", name)


def assert_no_forbidden_payload(payload: bytes, artifact: str, name: str) -> None:
    if LOCAL_ONLY_GOAL_PART.encode() in payload:
        raise SystemExit(f"{artifact} file {name!r} contains a forbidden local helper token")


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


if __name__ == "__main__":
    sys.exit(main())
