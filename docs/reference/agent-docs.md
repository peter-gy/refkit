---
description: Discover RefKit from marimo code mode, load its packaged Agent Skill, or read generated documentation as plain text.
---

# Agent-Readable Documentation

RefKit installs a [marimo code-mode](https://github.com/marimo-team/marimo/pull/10399) capability and an [Agent Plugin](https://agent-plugins.org/) beside its Python API. The capability directs notebook agents to the public RefKit objects. The Agent Plugin carries version-matched workflow instructions with the distribution.

## Discover RefKit in code mode

Marimo discovers capability metadata without importing RefKit:

```python
import marimo._code_mode as cm

assert cm.capabilities()["refkit"] == "refkit.agent"
```

Import the advertised module and render its dynamic help:

```python
import refkit.agent as refkit_agent

help(refkit_agent)
```

The help output starts with a complete normalized parse and render workflow, then prints the installed Agent Plugin tree and the matching `SKILL.md` path. Access those resources directly when an agent needs the complete workflow:

```python
plugin = refkit_agent.agent_plugin()
skill = refkit_agent.agent_skill()

print(plugin.tree(max_depth=3))
print(skill.body)
```

Marimo currently exposes code mode through the internal `marimo._code_mode` module. Treat discovery as a preview integration while using RefKit's public API for bibliography operations.

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
