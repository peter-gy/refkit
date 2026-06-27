from __future__ import annotations

from pathlib import Path
from typing import Any

from hatchling.builders.hooks.plugin.interface import BuildHookInterface


class BenchmarkBuildHook(BuildHookInterface):
    def initialize(self, version: str, build_data: dict[str, Any]) -> None:
        force_include = build_data.get("force_include", {})
        for path, distribution_path in list(force_include.items()):
            if distribution_path == ".gitignore" and Path(path).name == ".gitignore":
                del force_include[path]
