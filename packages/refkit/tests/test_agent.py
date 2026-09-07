from __future__ import annotations

import importlib
import pydoc
import subprocess
import sys
from importlib.metadata import version
from pathlib import Path
from types import ModuleType
from typing import get_type_hints

import agent_plugins
import pytest


@pytest.fixture(params=["refkit", "polars-refkit"])
def adapter(request: pytest.FixtureRequest) -> tuple[str, ModuleType]:
    distribution_name = str(request.param)
    module = importlib.import_module(f"{distribution_name.replace('-', '_')}.agent")
    return distribution_name, module


def test_capability_discovery_keeps_adapter_imports_lazy(
    adapter: tuple[str, ModuleType],
) -> None:
    distribution_name, _ = adapter
    package = distribution_name.replace("-", "_")
    program = f"""
import importlib
import sys
from importlib.metadata import distribution
import marimo._code_mode as code_mode

assert code_mode.capabilities()[{package!r}] == {f"{package}.agent"!r}
assert {f"{package}.agent"!r} not in sys.modules
assert "agent_plugins" not in sys.modules
importlib.import_module({package!r})
assert {f"{package}.agent"!r} not in sys.modules
assert "agent_plugins" not in sys.modules
entry = next(item for item in distribution({distribution_name!r}).entry_points
             if item.group == "marimo.agent.capability")
assert entry.load().__name__ == {f"{package}.agent"!r}
"""
    result = subprocess.run([sys.executable, "-c", program], capture_output=True, text=True)
    assert result.returncode == 0, result.stderr


def test_agent_resources_and_types_describe_the_installed_adapter(
    adapter: tuple[str, ModuleType],
) -> None:
    distribution_name, module = adapter
    plugin = module.agent_plugin()
    skill = module.agent_skill()
    resources = module.resources()

    assert plugin.manifest.name == distribution_name
    assert skill == plugin.skill(distribution_name)
    assert module.instructions() == skill.body.lstrip("\n")
    assert resources["references/inspect.md"].is_relative_to(skill.path)
    assert all(path.is_file() for path in resources.values())
    assert get_type_hints(module.agent_plugin)["return"] is agent_plugins.Plugin
    assert get_type_hints(module.agent_skill)["return"] is agent_plugins.Skill
    assert get_type_hints(module.resources)["return"] == dict[str, Path]
    assert module.AgentPluginError is agent_plugins.AgentPluginError

    help_text = pydoc.render_doc(module)
    assert version(distribution_name) in help_text
    assert str(skill.path) in help_text
    assert "references/inspect.md" in help_text


def test_installed_task_examples_execute_independently(
    adapter: tuple[str, ModuleType],
) -> None:
    distribution_name, _ = adapter
    result = subprocess.run(
        [sys.executable, "-m", "refkit_tests.agent_examples", distribution_name],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr


def test_agent_help_reports_resource_failure(
    adapter: tuple[str, ModuleType],
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    distribution_name, module = adapter

    def fail() -> agent_plugins.Plugin:
        raise agent_plugins.AgentPluginError("marker unavailable")

    monkeypatch.setattr(module, "agent_plugin", fail)
    rendered = pydoc.render_doc(module)
    assert "marker unavailable" in rendered
    assert f"Reinstall {distribution_name}" in rendered
    assert version(distribution_name) in rendered
    with pytest.raises(agent_plugins.AgentPluginError, match="marker unavailable"):
        module.instructions()


def test_agent_skill_reports_a_missing_skill(
    adapter: tuple[str, ModuleType],
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    _, module = adapter
    (tmp_path / "plugin.json").write_text('{"name": "empty"}\n')
    monkeypatch.setattr(module, "agent_plugin", lambda: agent_plugins.Plugin(tmp_path))
    with pytest.raises(agent_plugins.AgentPluginError, match="unavailable"):
        module.agent_skill()
