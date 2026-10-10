from __future__ import annotations

import json
import re
import shlex
import sys
from pathlib import Path

from scripts.architecture_contract import ROOT, _load


def _has_pypi_dependencies(value: object) -> bool:
    return isinstance(value, dict) and (
        "pypi-dependencies" in value
        or any(_has_pypi_dependencies(child) for child in value.values())
    )


def validate(root: Path = ROOT) -> list[str]:
    errors = []
    manifest = _load(root / "pixi.toml")
    dependencies = manifest.get("dependencies", {})
    floor = _load(root / "Cargo.toml")["workspace"]["package"]["rust-version"]
    rust = f"{floor}.0.*"
    errors.extend(
        f"pixi.toml: {name} must match the Rust floor ({rust})"
        for name in ("rust", "rust-std-wasm32-unknown-unknown")
        if dependencies.get(name) != rust
    )
    pnpm = json.loads((root / "docs/package.json").read_text())["packageManager"].split("@")[1]
    if dependencies.get("pnpm") != f"{pnpm}.*":
        errors.append("pixi.toml: pnpm must match docs/package.json")
    makefile = (root / "Makefile").read_text()
    for name, variable in (
        ("cargo-deny", "CARGO_DENY_VERSION"),
        ("cargo-shear", "CARGO_SHEAR_VERSION"),
    ):
        version = re.search(rf"^{variable} := (\S+)$", makefile, re.MULTILINE)
        if version is None or dependencies.get(name) != f"{version.group(1)}.*":
            errors.append(f"pixi.toml: {name} must match Makefile {variable}")
    cargo_lock = _load(root / "packages/refkit-js/rust/Cargo.lock")
    binding = next(
        package["version"] for package in cargo_lock["package"] if package["name"] == "wasm-bindgen"
    )
    command = manifest.get("tasks", {}).get("binding-tools", {}).get("cmd", "")
    args = shlex.split(command)
    if "--version" not in args or args[args.index("--version") + 1 :] != [binding]:
        errors.append("pixi.toml: binding-tools must match wasm-bindgen in the Wasm Cargo lock")
    if "--locked" not in args:
        errors.append("pixi.toml: binding-tools must use the upstream Cargo lock")
    if _has_pypi_dependencies(manifest):
        errors.append("pixi.toml: Python packages belong to uv.lock")
    return errors


def main() -> None:
    errors = validate()
    if errors:
        raise SystemExit("\n".join(errors))
    sys.stdout.write("Development environment contract passed\n")


if __name__ == "__main__":
    main()
