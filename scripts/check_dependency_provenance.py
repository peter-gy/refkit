from __future__ import annotations

import json
import subprocess
import sys
from typing import Any, cast

EXPECTED_PACKAGES = {
    "biblatex": "0.12.0",
    "citationberg": "0.7.0",
    "hayagriva": "0.10.0",
    "paste": "1.0.15",
    "serde_yaml": "0.9.34+deprecated",
    "unsafe-libyaml": "0.2.11",
}
EXPECTED_PASTE_DEPENDENTS = {"biblatex", "hayagriva"}
EXPECTED_SERDE_YAML_DEPENDENTS = {"hayagriva"}
EXPECTED_UNSAFE_LIBYAML_DEPENDENTS = {"serde_yaml"}
CRATES_IO_SOURCE = "registry+https://github.com/rust-lang/crates.io-index"
JsonObject = dict[str, Any]


def main() -> int:
    metadata = cargo_metadata()
    packages = cast(list[JsonObject], metadata["packages"])
    by_name: dict[str, list[JsonObject]] = {}
    for package in packages:
        by_name.setdefault(package["name"], []).append(package)

    for name, expected_version in EXPECTED_PACKAGES.items():
        matches = by_name.get(name, [])
        if len(matches) != 1:
            raise SystemExit(f"expected exactly one {name} package, found {len(matches)}")
        package = matches[0]
        actual_version = package["version"]
        if actual_version != expected_version:
            raise SystemExit(f"{name} version is {actual_version}, expected {expected_version}")
        source = package.get("source") or ""
        if source != CRATES_IO_SOURCE:
            raise SystemExit(f"{name} does not resolve from crates.io: {source!r}")

    assert_paste_advisory_path(metadata)
    assert_dependency_path(metadata, "serde_yaml", EXPECTED_SERDE_YAML_DEPENDENTS)
    assert_dependency_path(metadata, "unsafe-libyaml", EXPECTED_UNSAFE_LIBYAML_DEPENDENTS)
    print(json.dumps({name: EXPECTED_PACKAGES[name] for name in sorted(EXPECTED_PACKAGES)}))
    return 0


def cargo_metadata() -> JsonObject:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--format-version", "1"],
        capture_output=True,
        check=False,
        text=True,
    )
    if result.returncode != 0:
        raise SystemExit(result.stderr or result.stdout)
    return json.loads(result.stdout)


def assert_paste_advisory_path(metadata: JsonObject) -> None:
    assert_dependency_path(metadata, "paste", EXPECTED_PASTE_DEPENDENTS)


def assert_dependency_path(
    metadata: JsonObject, package_name: str, expected_dependents: set[str]
) -> None:
    packages = cast(list[JsonObject], metadata["packages"])
    package_ids = {package["id"] for package in packages if package["name"] == package_name}
    if len(package_ids) != 1:
        raise SystemExit(f"expected exactly one {package_name} package, found {len(package_ids)}")
    package_id = next(iter(package_ids))
    names_by_id = {package["id"]: package["name"] for package in packages}
    dependents = set()
    resolve = cast(JsonObject, metadata["resolve"])
    nodes = cast(list[JsonObject], resolve["nodes"])
    for node in nodes:
        deps = cast(list[JsonObject], node["deps"])
        dep_ids = {dep["pkg"] for dep in deps}
        if package_id in dep_ids:
            dependents.add(names_by_id[node["id"]])
    if dependents != expected_dependents:
        raise SystemExit(
            f"{package_name} is only accepted for dependents "
            f"{sorted(expected_dependents)}, found {sorted(dependents)}"
        )


if __name__ == "__main__":
    sys.exit(main())
