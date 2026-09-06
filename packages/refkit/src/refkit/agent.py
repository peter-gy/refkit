"""Use RefKit from code-mode agents."""

from __future__ import annotations

import sys as _sys
from textwrap import indent as _indent
from types import ModuleType as _ModuleType
from typing import TYPE_CHECKING as _TYPE_CHECKING

import agent_plugins as _agent_plugins

if _TYPE_CHECKING:  # pragma: no cover - typing-only public annotation names
    from pathlib import Path

    import agent_plugins

# Keep the future-feature binding out of the agent-facing module inventory.
del annotations

_DISTRIBUTION_NAME = "refkit"
_SKILL_NAME = "refkit"

AgentPluginError = _agent_plugins.AgentPluginError


def agent_plugin() -> agent_plugins.Plugin:
    """Return the Agent Plugin installed with this RefKit version.

    Raises:
        AgentPluginError: The installed distribution has no usable plugin marker.
    """

    return _agent_plugins.locate(_DISTRIBUTION_NAME)


def _agent_skill(plugin: agent_plugins.Plugin) -> agent_plugins.Skill:
    for skill in plugin.skills:
        if skill.path.name == _SKILL_NAME:
            return skill
    raise AgentPluginError("The RefKit Agent Plugin has no refkit skill. Reinstall refkit.")


def agent_skill() -> agent_plugins.Skill:
    """Return the packaged RefKit Agent Skill.

    Raises:
        AgentPluginError: The installed plugin or its RefKit skill cannot be resolved.
    """

    return _agent_skill(agent_plugin())


def instructions() -> str:
    """Return the installed RefKit Agent Skill instructions as Markdown."""

    return agent_skill().body.lstrip("\n")


def resources() -> dict[str, Path]:
    """Return installed skill files keyed by skill-relative path."""

    skill = agent_skill()
    return {path.relative_to(skill.path).as_posix(): path for path in skill.files}


def _sdk_help(summary: str) -> str:
    return f"""{summary}

Start with BibTeX source already in memory:

    import refkit as rk

    source = (
        "@article{{doe2024, author={{Doe, Jane}}, "
        "title={{Fast Citations}}, year={{2024}}}}"
    )
    library = rk.Library.parse_bibtex(source, recovery="report")
    rows = library.project(["key", "title", "date", "doi"])
    diagnostics = list(library.diagnostics)

    if diagnostics:
        print({{
            "status": "partial_recovery",
            "entries": rows,
            "diagnostics": diagnostics,
        }})
    else:
        document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
        rendered = document.render(
            [rk.Citation(id="result", citation="doe2024")]
        )
        print(rendered["result"].text)
        print(rendered.bibliography.text)

Inspect `diagnostics` before consuming entries recovered with
`recovery="report"`. Parsing still raises `RefkitError` when no entry survives
recovery. Use `recovery="error"` when malformed input must stop the operation.
Use `Library` for normalized lookup and rendering. Use `BibDocument` when source
order, comments, duplicate occurrences, and preserving writes matter. Use
`tidy_bibtex` for canonical formatting and inspect `TidyResult.warnings` before
consuming the formatted source.

Browse the published documentation map at:

    https://peter-gy.github.io/refkit/llms.txt
"""


def _module_help(summary: str) -> str:
    sdk = _sdk_help(summary)
    try:
        plugin = agent_plugin()
        skill = _agent_skill(plugin)
        tree = _indent(plugin.tree(max_depth=3, max_files=50), "    ")
    except AgentPluginError as error:
        return f"""{sdk}

The installed Agent Plugin could not be resolved: {error}
Reinstall refkit to restore its version-matched skill resources.
"""

    return f"""{sdk}

The installed Agent Plugin carries the complete RefKit workflow and resources
that match this package version:

{tree}

Read the RefKit skill instructions at:

    {skill / "SKILL.md"}

Load the instructions and known resource files programmatically:

    import refkit.agent as refkit_agent

    instructions = refkit_agent.instructions()
    resources = refkit_agent.resources()
    workflow = resources["references/workflows.md"].read_text()

Use `agent_plugin()` and `agent_skill()` when the underlying Agent Plugin
handles are required.
"""


__all__ = [
    "agent_plugin",
    "agent_skill",
    "instructions",
    "resources",
]


class _AgentModule(_ModuleType):
    @property
    def __doc__(self) -> str | None:  # pyrefly: ignore [bad-override]
        summary = self.__dict__.get("__doc__")
        if not isinstance(summary, str):  # pragma: no cover - interpreter module state
            return None
        return _module_help(summary)

    @__doc__.setter
    def __doc__(self, value: str | None) -> None:  # pragma: no cover - interpreter hook
        self.__dict__["__doc__"] = value


_sys.modules[__name__].__class__ = _AgentModule
