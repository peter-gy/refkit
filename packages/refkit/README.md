# refkit

`refkit` parses bibliography source, renders Citation Style Language citations and bibliographies, formats BibTeX, and preserves raw BibTeX for targeted field edits.

## Install

```bash
python -m pip install refkit
```

RefKit supports Python 3.11 through 3.14. Pip selects a compatible native wheel when one is available. A source-distribution build requires a Rust toolchain and a working Python build environment.

## Render A Citation

```python
import refkit as rk

library = rk.Library.parse_bibtex(
    """
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  year = {2024}
}
"""
)
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
rendered = document.render([rk.Citation("intro", "doe2024")])

print(rendered["intro"].text)
```

Expected output:

```text
(Doe, 2024)
```

`Library` owns normalized entries. `Document` stores the library, style, and locale inputs. Each render call receives the complete ordered citation document and returns named citations with their cited bibliography.

## Choose A Model

| Model | Use it for |
| --- | --- |
| `Library` | Normalized parsing, lookup, selection, projection, and rendering. |
| `BibDocument` | Source-order blocks, duplicate occurrences, byte spans, existing-field edits, and preserving writes. |

Every `Rendered` value exposes text, rendered HTML, and a structured tree. `TidyOptions` configures canonical BibTeX formatting and duplicate handling.

## Use from code-mode agents

Installing `refkit` also installs its [Agent Plugin](https://agent-plugins.org/)
and version-matched [Agent Skill](https://agentskills.io/specification). Any
code-mode agent that can execute Python can import `refkit.agent` and inspect
the workflow for the installed API:

```python
import refkit.agent as refkit_agent

help(refkit_agent)
```

For programmatic access, `instructions()` returns the packaged skill as Markdown
and `resources()` returns its known files by skill-relative name:

```python
instructions = refkit_agent.instructions()
resources = refkit_agent.resources()
workflow = resources["references/workflows.md"].read_text()
```

The agent workflow uses the same `Library`, `Document`, `BibDocument`, and tidy
APIs as other Python callers. Any code-mode environment that can execute Python
can use these imports directly. Clients that understand Agent Plugin metadata
can also discover the packaged resources.

[Marimo](https://marimo.io/) code mode also discovers `refkit.agent`
automatically through the capability entry point installed with the wheel.
Marimo currently exposes discovery through its internal preview code-mode
module. The [agent integration guide](https://peter-gy.github.io/refkit/reference/agent-docs)
records both the general Python path and marimo discovery.

## Documentation

- [Get started](https://github.com/peter-gy/refkit/blob/main/docs/get-started.md)
- [Python API](https://github.com/peter-gy/refkit/blob/main/docs/reference/python.md)
- [Raw BibTeX editing](https://github.com/peter-gy/refkit/blob/main/docs/guides/edit-bibtex.md)
- [Tidy options](https://github.com/peter-gy/refkit/blob/main/docs/reference/tidy-options.md)
- [Pyodide](https://github.com/peter-gy/refkit/blob/main/docs/pyodide.md)

RefKit is licensed under the Apache License 2.0.
