---
description: Discover installed RefKit tasks, execute version-matched examples, and retrieve focused agent documentation.
---

# Use RefKit with Agents

The Python and Polars packages each include an [Agent Plugin](https://agent-plugins.org/) with a version-matched [Agent Skill](https://agentskills.io/specification). An agent that can execute Python can discover the installed tasks:

```python
import refkit.agent

help(refkit.agent)
```

For a Polars environment:

```python
import polars_refkit.agent

help(polars_refkit.agent)
```

Module help identifies the installed version, summarizes the public API, and lists the packaged resources. Use those resources as the contract for the installed package. Website links describe the current documentation and can differ from an older installation.

## Choose a task

| Task | Python resource | Polars resource |
| --- | --- | --- |
| Inspect entries and recovery diagnostics | `references/inspect.md` | `references/inspect.md` |
| Render ordered citations or full bibliographies | `references/render.md` | `references/render.md` |
| Edit a raw BibTeX occurrence | `references/edit.md` | Use the Python object package. |
| Format, deduplicate, and generate keys | `references/tidy.md` | `references/tidy.md` |
| Look up object contracts | `references/contracts.md` | Use the installed expression signatures. |

Each task resource contains an independently executable example with concrete inputs and assertions. Start with the resource for the requested task, then inspect exact signatures or types when the workflow needs customization.

## Read the installed resources

```python
import refkit.agent as agent

instructions = agent.instructions()
resources = agent.resources()
inspection = resources["references/inspect.md"].read_text()
```

Both agent modules provide:

| Function | Result |
| --- | --- |
| `instructions()` | Skill instructions as Markdown. |
| `resources()` | Dictionary from skill-relative name to absolute `Path`. |
| `agent_plugin()` | The `agent_plugins.Plugin` handle. |
| `agent_skill()` | The `agent_plugins.Skill` handle. |

Resource functions raise `AgentPluginError` when the installation cannot supply its declared resources. Module help remains available and includes the recovery action. Reinstall the corresponding package when its resource payload is damaged.

## Discover capabilities in marimo

[Marimo](https://marimo.io/) discovers capability modules through package entry-point metadata. Install the relevant package in marimo's Python environment, then inspect discovery:

```python
import marimo._code_mode as cm

capabilities = cm.capabilities()
assert capabilities["refkit"] == "refkit.agent"
assert capabilities["polars_refkit"] == "polars_refkit.agent"
```

This example assumes both packages are installed. Each package contributes its own capability independently. Marimo's discovery API currently lives in the internal preview `marimo._code_mode` module. Agents can import the public capability modules directly in other Python execution environments.

## Fetch current documentation

RefKit publishes plain-text documentation using the [llms.txt convention](https://llmstxt.org/):

| Source | Task |
| --- | --- |
| [`llms.txt`](https://peter-gy.github.io/refkit/llms.txt) | Discover page descriptions and Markdown links. |
| [`llms-full.txt`](https://peter-gy.github.io/refkit/llms-full.txt) | Read the complete current public documentation. |
| A documentation route ending in `.md` | Read one focused page. |

For example:

```text
https://peter-gy.github.io/refkit/reference/python.md
https://peter-gy.github.io/refkit/guides/polars.md
```

The site and text views share authored Markdown, navigation, and deployment checks. Prefer installed skill resources for version-specific execution and focused website pages for current concepts or reference.
