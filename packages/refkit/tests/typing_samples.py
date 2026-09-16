"""Consumer examples checked by ty and pyrefly without starting native runtimes."""

from pathlib import Path

import agent_plugins
from typing_extensions import assert_type

import polars_refkit.agent as polars_agent
import refkit as rk
import refkit.agent as agent
from refkit.types import (
    CitePurpose,
    Diagnostic,
    RawBlock,
    RenderedTree,
    ResolvedBibEntry,
    StyleMetadata,
    TidyRename,
)


def consume_structured_results(
    library: rk.Library,
    raw: rk.BibDocument,
    rendered: rk.Rendered,
    tidied: rk.TidyResult,
) -> None:
    assert_type(library.diagnostics, list[Diagnostic])
    assert_type(library.get_many(["key"]), list[rk.Entry])
    assert_type(raw.blocks, list[RawBlock])
    assert_type(raw.resolve(), list[ResolvedBibEntry])
    assert_type(rendered.tree, RenderedTree)
    assert_type(tidied.renames, list[TidyRename])
    assert_type(rk.Style.list(), list[StyleMetadata])
    assert_type(rk.Cite("key", purpose="author").purpose, CitePurpose)


def discover_agent_resources() -> None:
    assert_type(agent.agent_plugin(), agent_plugins.Plugin)
    assert_type(agent.agent_skill(), agent_plugins.Skill)
    assert_type(agent.resources(), dict[str, Path])
    assert_type(polars_agent.agent_plugin(), agent_plugins.Plugin)
    assert_type(polars_agent.agent_skill(), agent_plugins.Skill)
    assert_type(polars_agent.resources(), dict[str, Path])
