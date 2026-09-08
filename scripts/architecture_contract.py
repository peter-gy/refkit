from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Any

if sys.version_info >= (3, 11):
    import tomllib
else:
    import tomli as tomllib

ROOT = Path(__file__).parents[1]
PORTABLE_CORE = Path("crates/refkit-core/Cargo.toml")
NATIVE_ADAPTER = Path("packages/refkit/rust/Cargo.toml")
JS_ADAPTER = Path("packages/refkit-js/rust/Cargo.toml")
POLARS_ADAPTER = Path("packages/polars-refkit/rust/Cargo.toml")
REFKIT_PROJECT = Path("packages/refkit/pyproject.toml")
NATIVE_PACKAGES = (REFKIT_PROJECT, Path("packages/polars-refkit/pyproject.toml"))
ENGINE_DEPENDENCIES = ("biblatex", "hayagriva")
HAYAGRIVA_REPOSITORY = "https://github.com/typst/hayagriva"
HAYAGRIVA_REVISION = "e7a9e7cecbbf774fd0d5226faeec12a7f8481a2e"
CITATIONBERG_REPOSITORY = "https://github.com/typst/citationberg"
CITATIONBERG_REVISION = "06a591e2f237d25e1dfdedac3f3d1494c496c52d"
REFKIT_RUNTIME_DEPENDENCIES = ["agent-plugins==0.2.0"]
CARGO_LOCKS = (
    Path("Cargo.lock"),
    Path("packages/polars-refkit/rust/Cargo.lock"),
    Path("packages/refkit-js/rust/Cargo.lock"),
)
ALLOWED_DEPENDENCIES = {
    PORTABLE_CORE: {
        "dependencies": {"biblatex", "hayagriva", "indexmap", "quick-xml"},
        "dev-dependencies": {"serde", "serde_yaml", "walkdir"},
    },
    NATIVE_ADAPTER: {
        "dependencies": {"pyo3", "refkit-core", "serde_json"},
        "dev-dependencies": {"pyo3"},
    },
    JS_ADAPTER: {
        "dependencies": {"refkit-core", "serde", "serde_json", "wasm-bindgen"},
        "dev-dependencies": set(),
    },
    POLARS_ADAPTER: {
        "dependencies": {
            "polars",
            "polars-core",
            "pyo3",
            "pyo3-polars",
            "refkit-core",
            "serde",
        },
        "dev-dependencies": set(),
    },
}
FORBIDDEN_CORE_SOURCE = {
    "host standard library": re.compile(
        r"\b(?:std\s*::\s*(?:env|fs|net|path|process)|"
        r"use\s+std\s*::\s*\{[^}]*\b(?:env|fs|net|path|process)\b)",
        re.DOTALL,
    ),
    "adapter dependency": re.compile(r"\b(?:pyo3|polars(?:_core)?)\b"),
    "upstream type in a public declaration": re.compile(
        r"^\s*pub\s+(?:fn|struct|enum|type|trait|use)\b[^;{]*"
        r"\b(?:hayagriva|biblatex|serde_json)\b",
        re.MULTILINE,
    ),
}


def _load(path: Path) -> dict[str, Any]:
    with path.open("rb") as file:
        return tomllib.load(file)


def _dependency_errors(
    manifest_path: Path,
    manifest: dict[str, Any],
    allowed: dict[str, set[str]],
    workspace: dict[str, Any] | None = None,
) -> list[str]:
    errors = []
    tables = [
        ("", manifest),
        *[(f"target.{target}.", table) for target, table in manifest.get("target", {}).items()],
    ]
    for prefix, table in tables:
        for section in ("dependencies", "dev-dependencies", "build-dependencies"):
            for alias, specification in table.get(section, {}).items():
                if isinstance(specification, dict) and specification.get("workspace"):
                    specification = (workspace or {}).get(alias, specification)
                package = (
                    specification.get("package", alias)
                    if isinstance(specification, dict)
                    else alias
                )
                if package not in allowed.get(section, set()):
                    errors.append(
                        f"{manifest_path} contains unclassified {prefix}{section}: {package}"
                    )
    return errors


def _refkit_dependency_errors(manifest: dict[str, Any]) -> list[str]:
    dependencies = manifest.get("project", {}).get("dependencies", [])
    if dependencies == REFKIT_RUNTIME_DEPENDENCIES:
        return []
    return [
        "packages/refkit runtime dependencies must contain only "
        f"{', '.join(REFKIT_RUNTIME_DEPENDENCIES)}"
    ]


def _core_source_errors(root: Path) -> list[str]:
    errors = []
    source_root = root / PORTABLE_CORE.parent / "src"
    for path in sorted(source_root.rglob("*.rs")):
        source = path.read_text(encoding="utf-8")
        errors.extend(
            f"{path.relative_to(root)} reaches a host boundary through {boundary}"
            for boundary, pattern in FORBIDDEN_CORE_SOURCE.items()
            if pattern.search(source)
        )
    return errors


def _engine_dependency_errors(core: dict[str, Any], locks: dict[Path, dict[str, Any]]) -> list[str]:
    errors = []
    expected_versions = {}
    dependencies = core.get("dependencies", {})

    for name in ENGINE_DEPENDENCIES:
        dependency = dependencies.get(name)
        version = dependency.get("version") if isinstance(dependency, dict) else None
        if not isinstance(version, str) or not version.startswith("=") or len(version) == 1:
            errors.append(f"portable core must pin {name} to one exact release with =<version>")
            continue
        expected_versions[name] = version[1:]
        if name == "hayagriva":
            if (
                dependency.get("git") != HAYAGRIVA_REPOSITORY
                or dependency.get("rev") != HAYAGRIVA_REVISION
                or version != "=0.10.1"
            ):
                errors.append(
                    "portable core must use the audited Hayagriva 0.10.1 revision "
                    + HAYAGRIVA_REVISION
                )
        elif isinstance(dependency, dict) and any(key in dependency for key in ("git", "path")):
            errors.append(f"portable core must resolve {name} from crates.io")

    for lock_path, lock in locks.items():
        packages = lock.get("package", [])
        for name, expected in expected_versions.items():
            resolved = [package for package in packages if package.get("name") == name]
            if len(resolved) != 1:
                errors.append(f"{lock_path} must resolve exactly one {name} package")
                continue

            package = resolved[0]
            if package.get("version") != expected:
                errors.append(f"{lock_path} must resolve {name} {expected}")
            source = package.get("source")
            expected_source = (
                f"git+{HAYAGRIVA_REPOSITORY}?rev={HAYAGRIVA_REVISION}#{HAYAGRIVA_REVISION}"
                if name == "hayagriva"
                else "registry+https://github.com/rust-lang/crates.io-index"
            )
            if source != expected_source:
                errors.append(f"{lock_path} must resolve {name} from {expected_source}")

    return errors


def _xml_dependency_errors(
    core: dict[str, Any],
    workspaces: dict[Path, dict[str, Any]],
    locks: dict[Path, dict[str, Any]],
) -> list[str]:
    errors = []
    expected_patch = {
        "crates-io": {
            "citationberg": {"git": CITATIONBERG_REPOSITORY, "rev": CITATIONBERG_REVISION}
        }
    }
    for path, manifest in workspaces.items():
        if manifest.get("patch") != expected_patch:
            errors.append(
                f"{path} must patch citationberg to audited revision {CITATIONBERG_REVISION}"
            )

    xml = core.get("dependencies", {}).get("quick-xml")
    if xml != "=0.41.0":
        errors.append("portable core must pin quick-xml =0.41.0 from crates.io")

    source = f"git+{CITATIONBERG_REPOSITORY}?rev={CITATIONBERG_REVISION}#{CITATIONBERG_REVISION}"
    for path, lock in locks.items():
        packages = lock.get("package", [])
        citationberg = [package for package in packages if package.get("name") == "citationberg"]
        if (
            len(citationberg) != 1
            or citationberg[0].get("version") != "0.7.0"
            or citationberg[0].get("source") != source
        ):
            errors.append(
                f"{path} must resolve one citationberg 0.7.0 from audited revision "
                f"{CITATIONBERG_REVISION}"
            )
        xml_packages = [package for package in packages if package.get("name") == "quick-xml"]
        if not xml_packages:
            errors.append(f"{path} must resolve quick-xml")
        for package in xml_packages:
            version = package.get("version", "")
            release = re.fullmatch(r"(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", version)
            if (
                release is None
                or tuple(map(int, release.groups())) < (0, 41, 0)
                or package.get("source") != "registry+https://github.com/rust-lang/crates.io-index"
            ):
                errors.append(f"{path} must resolve quick-xml >=0.41.0 from crates.io")
    return errors


def check_contract(root: Path) -> list[str]:
    workspace = _load(root / "Cargo.toml")
    core = _load(root / PORTABLE_CORE)
    native = _load(root / NATIVE_ADAPTER)
    polars = _load(root / POLARS_ADAPTER)
    errors = []

    manifests = {
        PORTABLE_CORE: core,
        NATIVE_ADAPTER: native,
        POLARS_ADAPTER: polars,
        JS_ADAPTER: _load(root / JS_ADAPTER),
    }
    for manifest_path, allowed in ALLOWED_DEPENDENCIES.items():
        errors.extend(
            _dependency_errors(
                manifest_path,
                manifests[manifest_path],
                allowed,
                workspace["workspace"]["dependencies"],
            )
        )
    errors.extend(_core_source_errors(root))

    locks = {relative_path: _load(root / relative_path) for relative_path in CARGO_LOCKS}
    errors.extend(_engine_dependency_errors(core, locks))
    errors.extend(
        _xml_dependency_errors(
            core,
            {
                Path("Cargo.toml"): workspace,
                POLARS_ADAPTER: polars,
                JS_ADAPTER: manifests[JS_ADAPTER],
            },
            locks,
        )
    )

    members = set(workspace["workspace"]["members"])
    expected_members = {"crates/refkit-core", "packages/refkit/rust"}
    if members != expected_members:
        errors.append("root workspace must contain the portable core and Python adapter")
    if "packages/polars-refkit/rust" in members:
        errors.append("Polars adapter must remain in its package-local Cargo workspace")

    native_dependencies = native.get("dependencies", {})
    if not native_dependencies.get("pyo3", {}).get("workspace"):
        errors.append("native adapter must use the workspace PyO3 dependency")
    if not native_dependencies.get("refkit-core", {}).get("workspace"):
        errors.append("native adapter must use the workspace portable core dependency")

    if "workspace" not in polars:
        errors.append("Polars adapter must declare its package-local workspace")
    polars_core = polars.get("dependencies", {}).get("refkit-core", {})
    core_path = (root / POLARS_ADAPTER.parent / polars_core.get("path", "")).resolve()
    if core_path != (root / PORTABLE_CORE.parent).resolve():
        errors.append("Polars adapter must depend on the shared portable core")

    javascript = manifests[JS_ADAPTER]
    if "workspace" not in javascript:
        errors.append("JavaScript adapter must declare its package-local workspace")
    javascript_core = javascript.get("dependencies", {}).get("refkit-core", {})
    javascript_core_path = (root / JS_ADAPTER.parent / javascript_core.get("path", "")).resolve()
    if javascript_core_path != (root / PORTABLE_CORE.parent).resolve():
        errors.append("JavaScript adapter must depend on the shared portable core")

    refkit = _load(root / REFKIT_PROJECT)
    errors.extend(_refkit_dependency_errors(refkit))

    for relative_path in NATIVE_PACKAGES:
        pyproject = _load(root / relative_path)
        maturin = pyproject.get("tool", {}).get("maturin", {})
        if maturin.get("locked") is not True:
            errors.append(f"{relative_path} must enable locked Maturin builds")
        excludes = maturin.get("exclude", [])
        errors.extend(
            f"{relative_path} must exclude {generated} artifacts"
            for generated in ("__pycache__", ".pyc", ".pyo")
            if not any(generated in pattern for pattern in excludes)
        )

    return errors


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Validate refkit workspace ownership boundaries.")
    parser.add_argument(
        "--root",
        type=Path,
        default=ROOT,
        help="Repository root. Defaults to the checkout containing this script.",
    )
    return parser.parse_args()


def main() -> None:
    root = _parse_args().root.resolve()
    errors = check_contract(root)
    if errors:
        raise SystemExit("Architecture contract failed:\n" + "\n".join(f"- {e}" for e in errors))
    sys.stdout.write("Architecture contract passed\n")


if __name__ == "__main__":
    main()
