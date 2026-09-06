---
description: Use RefKit from code-mode agents, load its version-matched Agent Skill, or read generated documentation as plain text.
---

# Agent integration

The `refkit` wheel contains the Python library, an [Agent Plugin](https://agent-plugins.org/), and an [Agent Skill](https://agentskills.io/specification). Installing `refkit` places the library and its version-matched agent instructions in the same Python environment, so an agent reads workflows for the API version it can call.

## Start with module help

Any code-mode agent that can execute Python can import `refkit.agent`. Render its dynamic help for a complete parse and render workflow plus the installed resource paths:

```python
import refkit.agent as refkit_agent

help(refkit_agent)
```

The help remains available when packaged-resource lookup fails and includes the reinstall action. Catch `refkit_agent.AgentPluginError` when code calls a resource function directly and needs to handle a damaged installation.

## Load packaged instructions and resources

`instructions()` returns the installed RefKit skill as Markdown. `resources()` returns the known skill files by relative name, with absolute `Path` values:

```python
instructions = refkit_agent.instructions()
resources = refkit_agent.resources()
workflow = resources["references/workflows.md"].read_text()
contracts = resources["references/contracts.md"].read_text()
```

The agent workflow uses the same `Library`, `Document`, `BibDocument`, and tidy APIs as other Python callers. Any code-mode environment that can execute Python can use these imports directly. Clients that understand Agent Plugin metadata can also discover the packaged resources.

Call `agent_plugin()` or `agent_skill()` when code needs the underlying `agent_plugins.Plugin` or `agent_plugins.Skill` handle.

## Discover RefKit in marimo

[Marimo](https://marimo.io/) is one environment that discovers the capability module automatically. Its code mode reads the entry-point metadata without importing RefKit:

Install `refkit` in the Python environment that runs marimo, then inspect the capability map:

```python
import marimo._code_mode as cm

assert cm.capabilities()["refkit"] == "refkit.agent"
```

Marimo currently exposes capability discovery through the internal preview `marimo._code_mode` module. The discovered `refkit.agent` module and the bibliography APIs it documents are regular Python modules.

## Read generated documentation

RefKit publishes its documentation as plain text for large language model (LLM) clients. The generated files follow the [llms.txt convention](https://llmstxt.org/).

## Choose a context source

| Source | Use it for |
| --- | --- |
| [`llms.txt`](https://peter-gy.github.io/refkit/llms.txt) | Discover page titles, descriptions, and raw Markdown links. |
| [`llms-full.txt`](https://peter-gy.github.io/refkit/llms-full.txt) | Load the complete public documentation as one text document. |
| A route ending in `.md` | Read one page without site navigation or rendered HTML. |

Start with `llms.txt` when the client can fetch pages on demand. Use `llms-full.txt` when the complete document fits the task's context budget.

For a focused lookup, append `.md` to a documentation route:

```text
https://peter-gy.github.io/refkit/reference/python.md
https://peter-gy.github.io/refkit/guides/polars.md
```

The build derives these files from the same Markdown source as the website. Titles, descriptions, page coverage, deployment paths, and generated links are checked before the GitHub Pages artifact is uploaded.
