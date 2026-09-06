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

## Use From Marimo Code Mode

RefKit registers `refkit.agent` as a marimo code-mode capability. Import the module named by `marimo._code_mode.capabilities()`, then call `help` for a version-matched workflow and packaged Agent Skill:

```python
import refkit.agent as refkit_agent

help(refkit_agent)
```

The capability guides agents to the same `Library`, `Document`, `BibDocument`, and tidy APIs used by regular Python callers.

Marimo exposes capability discovery through its internal preview code-mode module. The [agent-readable documentation](https://peter-gy.github.io/refkit/reference/agent-docs) records the current discovery and resource contract.

## Documentation

- [Get started](https://github.com/peter-gy/refkit/blob/main/docs/get-started.md)
- [Python API](https://github.com/peter-gy/refkit/blob/main/docs/reference/python.md)
- [Raw BibTeX editing](https://github.com/peter-gy/refkit/blob/main/docs/guides/edit-bibtex.md)
- [Tidy options](https://github.com/peter-gy/refkit/blob/main/docs/reference/tidy-options.md)
- [Pyodide](https://github.com/peter-gy/refkit/blob/main/docs/pyodide.md)

RefKit is licensed under the Apache License 2.0.
