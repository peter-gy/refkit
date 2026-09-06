from __future__ import annotations

import pydoc
import subprocess
import sys
from pathlib import Path

import agent_plugins
import pytest

import refkit.agent as refkit_agent

ROOT = Path(__file__).parents[3]
PLUGIN_FILES = {
    "plugin.json",
    "skills/refkit/SKILL.md",
    "skills/refkit/agents/openai.yaml",
    "skills/refkit/references/contracts.md",
    "skills/refkit/references/workflows.md",
}
SKILL_FILES = {
    "SKILL.md",
    "agents/openai.yaml",
    "references/contracts.md",
    "references/workflows.md",
}


def test_agent_module_exports_resource_access() -> None:
    assert refkit_agent.__all__ == [
        "agent_plugin",
        "agent_skill",
        "instructions",
        "resources",
    ]
    assert refkit_agent.AgentPluginError is agent_plugins.AgentPluginError
    assert "annotations" not in dir(refkit_agent)


def test_marimo_code_mode_discovers_and_loads_refkit_on_demand() -> None:
    result = subprocess.run(
        [
            sys.executable,
            "-c",
            """
import pydoc
import sys
from importlib.metadata import distribution

import marimo._code_mode as code_mode

assert code_mode.capabilities()["refkit"] == "refkit.agent"
assert "agent_plugins" not in sys.modules
assert "refkit.agent" not in sys.modules
assert "refkit    import refkit.agent" in pydoc.render_doc(code_mode)

capabilities = [
    entry
    for entry in distribution("refkit").entry_points
    if entry.group == "marimo.agent.capability"
]
assert [(entry.name, entry.value) for entry in capabilities] == [
    ("refkit", "refkit.agent")
]
module = capabilities[0].load()
assert module.__name__ == "refkit.agent"
assert "Library.parse_bibtex" in pydoc.render_doc(module)
""",
        ],
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0, result.stderr


def test_root_import_leaves_agent_capability_lazy() -> None:
    result = subprocess.run(
        [
            sys.executable,
            "-c",
            """
import sys
import refkit

assert "agent_plugins" not in sys.modules
assert "refkit.agent" not in sys.modules
""",
        ],
        capture_output=True,
        text=True,
        check=False,
    )

    assert result.returncode == 0, result.stderr


def test_agent_plugin_exposes_packaged_refkit_skill() -> None:
    plugin = refkit_agent.agent_plugin()
    skill = refkit_agent.agent_skill()

    assert plugin.manifest.name == "refkit"
    assert skill.path.name == "refkit"
    assert "name: refkit" in skill.frontmatter
    assert "refkit.Library" in skill.body
    assert "refkit.BibDocument" in skill.body
    assert {path.relative_to(plugin.path).as_posix() for path in plugin.files} == PLUGIN_FILES


def test_agent_module_returns_instructions_and_known_resources() -> None:
    skill = refkit_agent.agent_skill()
    resources = refkit_agent.resources()

    assert refkit_agent.instructions() == skill.body.lstrip("\n")
    assert refkit_agent.instructions().startswith("# RefKit\n")
    assert set(resources) == SKILL_FILES
    assert all(path.is_file() for path in resources.values())
    assert all(path.is_relative_to(skill.path) for path in resources.values())


def test_agent_plugin_build_plan_contains_authored_resources() -> None:
    plan = agent_plugins.build_plan(ROOT / "packages/refkit")

    assert {mapping.target.as_posix() for mapping in plan.files} == PLUGIN_FILES


def test_agent_module_help_points_to_sdk_and_installed_resources() -> None:
    plugin = refkit_agent.agent_plugin()
    skill = refkit_agent.agent_skill()
    rendered = pydoc.render_doc(refkit_agent)

    assert str(plugin.path) in rendered
    assert str(skill / "SKILL.md") in rendered
    assert "Library.parse_bibtex" in rendered
    assert "diagnostics = list(library.diagnostics)" in rendered
    assert '"status": "partial_recovery"' in rendered
    assert "Document(library" in rendered
    assert 'Citation(id="result", citation="doe2024")' in rendered
    assert '"@article{doe2024, author={Doe, Jane}, "' in rendered
    assert "refkit_agent.instructions()" in rendered
    assert 'resources["references/workflows.md"].read_text()' in rendered
    assert "https://peter-gy.github.io/refkit/llms.txt" in rendered
    assert "Method resolution order" not in rendered


def test_agent_module_help_preserves_sdk_when_plugin_lookup_fails(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fail() -> agent_plugins.Plugin:
        raise refkit_agent.AgentPluginError("marker unavailable")

    monkeypatch.setattr(refkit_agent, "agent_plugin", fail)
    rendered = pydoc.render_doc(refkit_agent)

    assert "Library.parse_bibtex" in rendered
    assert "marker unavailable" in rendered
    assert "Reinstall refkit" in rendered


def test_agent_skill_reports_missing_packaged_skill(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    (tmp_path / "plugin.json").write_text('{"name": "refkit"}\n')
    other = tmp_path / "skills" / "other"
    other.mkdir(parents=True)
    (other / "SKILL.md").write_text(
        "---\nname: other\ndescription: Another skill.\n---\n\n# Other\n"
    )
    monkeypatch.setattr(refkit_agent, "agent_plugin", lambda: agent_plugins.Plugin(tmp_path))

    with pytest.raises(agent_plugins.AgentPluginError, match="has no refkit skill"):
        refkit_agent.agent_skill()
