"""Use RefKit from notebook agents."""

from __future__ import annotations

import sys
from textwrap import indent
from types import ModuleType

import agent_plugins

_DISTRIBUTION_NAME = "refkit"
_SKILL_NAME = "refkit"


def agent_plugin() -> agent_plugins.Plugin:
    """Return the Agent Plugin installed with this RefKit version.

    Raises:
        AgentPluginError: The installed distribution has no usable plugin marker.
    """

    return agent_plugins.locate(_DISTRIBUTION_NAME)


def _agent_skill(plugin: agent_plugins.Plugin) -> agent_plugins.Skill:
    for skill in plugin.skills:
        if skill.path.name == _SKILL_NAME:
            return skill
    raise agent_plugins.AgentPluginError(
        "The RefKit Agent Plugin has no refkit skill. Reinstall refkit."
    )


def agent_skill() -> agent_plugins.Skill:
    """Return the packaged RefKit Agent Skill.

    Raises:
        AgentPluginError: The installed plugin or its RefKit skill cannot be resolved.
    """

    return _agent_skill(agent_plugin())


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

    document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
    rendered = document.render([rk.Citation("result", "doe2024")])
    print(rendered["result"].text)
    print(rendered.bibliography.text)

Use `Library` for normalized lookup and rendering. Use `BibDocument` when
source order, comments, duplicate occurrences, and preserving writes matter.
Use `tidy_bibtex` for canonical formatting and inspect `TidyResult.warnings`
before consuming the formatted source.

Browse the published documentation map at:

    https://peter-gy.github.io/refkit/llms.txt
"""


def _module_help(summary: str) -> str:
    sdk = _sdk_help(summary)
    try:
        plugin = agent_plugin()
        skill = _agent_skill(plugin)
        tree = indent(plugin.tree(max_depth=3, max_files=50), "    ")
    except agent_plugins.AgentPluginError as error:
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

Traverse the same resources programmatically:

    import refkit.agent as refkit_agent

    resources = refkit_agent.agent_plugin()
    skill = refkit_agent.agent_skill()
    print(resources)
    print(skill.body)
"""


__all__ = ["agent_plugin", "agent_skill"]


class _AgentModule(ModuleType):
    @property
    def __doc__(self) -> str | None:  # pyrefly: ignore [bad-override]
        summary = self.__dict__.get("__doc__")
        if not isinstance(summary, str):  # pragma: no cover - interpreter module state
            return None
        return _module_help(summary)

    @__doc__.setter
    def __doc__(self, value: str | None) -> None:  # pragma: no cover - interpreter hook
        self.__dict__["__doc__"] = value


sys.modules[__name__].__class__ = _AgentModule
