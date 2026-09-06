---
description: Give coding agents a page index, one complete RefKit context file, or raw Markdown for a specific documentation route.
---

# Agent-Readable Documentation

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
