"""Use RefKit from code-mode agents."""

import sys as _sys
from importlib.metadata import version as _version
from pathlib import Path as _Path
from textwrap import indent as _indent
from types import ModuleType as _ModuleType

import agent_plugins as _agent_plugins

_DISTRIBUTION_NAME = "refkit"
_SKILL_NAME = "refkit"

AgentPluginError = _agent_plugins.AgentPluginError


def agent_plugin() -> _agent_plugins.Plugin:
    """Return the Agent Plugin installed with this RefKit version.

    Raises:
        AgentPluginError: The installed distribution has no usable plugin marker.
    """

    return _agent_plugins.locate(_DISTRIBUTION_NAME)


def agent_skill() -> _agent_plugins.Skill:
    """Return the packaged RefKit Agent Skill.

    Raises:
        AgentPluginError: The installed plugin or its RefKit skill cannot be resolved.
    """

    return agent_plugin().skill(_SKILL_NAME)


def instructions() -> str:
    """Return the installed RefKit Agent Skill instructions as Markdown."""

    return agent_skill().body.lstrip("\n")


def resources() -> dict[str, _Path]:
    """Return installed skill files keyed by skill-relative path."""

    skill = agent_skill()
    return {path.relative_to(skill.path).as_posix(): path for path in skill.files}


def _module_help(summary: str) -> str:
    introduction = f"""{summary}

Installed refkit version: {_version(_DISTRIBUTION_NAME)}.

Use `refkit.Library` to inspect normalized bibliography data, `refkit.Document`
to render ordered citations, and `refkit.BibDocument` to edit raw BibTeX.

    import refkit.agent as agent

    print(agent.instructions())
    print(agent.resources()["references/inspect.md"].read_text(encoding="utf-8"))

Packaged instructions describe the installed API. Read the task resource before
executing its example. The website at https://peter-gy.github.io/refkit/ describes
the current published API and can differ from this installed version.
"""
    try:
        skill = agent_skill()
        tree = _indent(skill.tree(), "    ")
    except AgentPluginError as error:
        return f"{introduction}\nAgent Plugin unavailable: {error}\nReinstall refkit.\n"
    return f"{introduction}\nInstalled task resources:\n\n{tree}\n"


__all__ = [
    "AgentPluginError",
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
