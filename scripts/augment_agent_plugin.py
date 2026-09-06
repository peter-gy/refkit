"""Add RefKit's Agent Plugin to wheels produced by direct Maturin builds."""

from __future__ import annotations

import argparse
import shutil
import sys
import tempfile
from contextlib import chdir
from pathlib import Path

from agent_plugins.build import BuildBackend

from scripts.distribution_contract import distribution_paths

ROOT = Path(__file__).parents[1]
REFKIT_PROJECT = ROOT / "packages" / "refkit"
_BACKEND = BuildBackend("scripts._prebuilt_wheel_backend")


def augment_wheel(wheel: Path, *, project: Path = REFKIT_PROJECT) -> None:
    """Add the configured Agent Plugin to one existing wheel atomically."""

    source = wheel.resolve(strict=True)
    if source.suffix != ".whl":
        raise ValueError(f"Expected a wheel path, received {source}")

    with tempfile.TemporaryDirectory(
        dir=source.parent,
        prefix=f".{source.name}.agent-plugin.",
    ) as directory:
        staged = Path(directory) / source.name
        shutil.copy2(source, staged)
        with chdir(project.resolve(strict=True)):
            filename = _BACKEND.build_wheel(directory)
        if filename != source.name:
            raise RuntimeError(
                f"Agent Plugin backend returned {filename!r}, expected {source.name!r}"
            )
        staged.replace(source)


def main(argv: list[str] | None = None) -> int:
    """Augment direct Maturin wheels with RefKit's Agent Plugin."""

    parser = argparse.ArgumentParser(
        description="Package RefKit Agent Plugin resources into existing wheels."
    )
    parser.add_argument("wheels", nargs="+")
    arguments = parser.parse_args(argv)
    wheels, unmatched = distribution_paths(arguments.wheels)
    if unmatched:
        parser.error(f"no wheel files matched: {', '.join(unmatched)}")
    for wheel in wheels:
        augment_wheel(wheel)
        sys.stdout.write(f"{wheel}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
