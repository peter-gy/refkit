<p align="center">
  <a href="https://peter-gy.github.io/refkit/">
    <img alt="RefKit" src="https://peter-gy.github.io/refkit/brand/refkit-lockup-horizontal-light-transparent.svg" width="420">
  </a>
</p>

<p align="center">
  Parse, cite, tidy, and edit bibliography data from Python.
</p>

<p align="center">
  <a href="https://peter-gy.github.io/refkit/"><strong>Documentation</strong></a> ·
  <a href="https://github.com/peter-gy/refkit"><strong>Source</strong></a> ·
  <a href="https://pypi.org/project/polars-refkit/"><strong>Polars expressions</strong></a>
</p>

<p align="center">
  <a href="https://pypi.org/project/refkit/"><img alt="PyPI version" src="https://img.shields.io/pypi/v/refkit.svg"></a>
  <a href="https://pypi.org/project/refkit/"><img alt="Supported Python versions" src="https://img.shields.io/pypi/pyversions/refkit.svg"></a>
  <a href="https://github.com/peter-gy/refkit/actions/workflows/ci.yml"><img alt="CI status" src="https://github.com/peter-gy/refkit/actions/workflows/ci.yml/badge.svg?branch=main"></a>
  <a href="https://github.com/peter-gy/refkit/blob/main/LICENSE"><img alt="Apache-2.0 license" src="https://img.shields.io/pypi/l/refkit.svg"></a>
</p>

> **Alpha:** Public APIs may change before 1.0.

`refkit` turns [BibTeX](https://ctan.org/pkg/bibtex),
[BibLaTeX](https://ctan.org/pkg/biblatex), and
[Hayagriva](https://github.com/typst/hayagriva) YAML into normalized
bibliography records. It renders citations and bibliographies with
[Citation Style Language](https://citationstyles.org/) styles, produces plain
text, HTML, or structured trees, and preserves raw BibTeX for targeted edits.

## Render a citation

Add `refkit` to a Python 3.11 through 3.14 environment:

```bash
python -m pip install refkit
```

Parse a bibliography, load the APA style, and render one citation:

```python
import refkit as rk

library = rk.Library.parse_bibtex(
    """
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  journal = {Journal of Citation Tests},
  year = {2024}
}
"""
)
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
rendered = document.render([rk.Citation(id="intro", citation="doe2024")])

print(rendered["intro"].text)
print(rendered.bibliography.text)
```

```text
(Doe, 2024)
Doe, J. (2024). Fast Citations. Journal of Citation Tests.
```

`Library` owns normalized entries and parser diagnostics. `Document` combines
a library, style, and locale for one ordered render call. Each `Rendered` value
exposes text, HTML, and a structured render tree.

## Choose a bibliography model

| Model | Preserves | Use it for |
| --- | --- | --- |
| `Library` | Normalized entries and diagnostics | Lookup, projection, citation rendering, and full bibliographies. |
| `BibDocument` | Source-order blocks, duplicate occurrences, and byte spans | Raw BibTeX inspection, targeted field edits, and source-preserving writes. |

`TidyOptions` configures canonical BibTeX formatting and duplicate handling for
`tidy_bibtex()` and `BibDocument.tidy()`.

## Use with code-mode agents

Installing `refkit` also installs its
[Agent Plugin](https://agent-plugins.org/) and version-matched
[Agent Skill](https://agentskills.io/specification). Any code-mode agent that
can execute Python can inspect the installed workflow:

```python
import refkit.agent as refkit_agent

help(refkit_agent)
instructions = refkit_agent.instructions()
resources = refkit_agent.resources()
```

[Agent integration](https://peter-gy.github.io/refkit/reference/agent-docs)
documents capability discovery, packaged resources, and generated text views.

## Documentation

- [Get started](https://peter-gy.github.io/refkit/get-started) reaches a complete render result.
- [Parsing guide](https://peter-gy.github.io/refkit/guides/parse-bibliographies) covers normalized input and recovery workflows.
- [Raw BibTeX editing](https://peter-gy.github.io/refkit/guides/edit-bibtex) covers occurrence selection and preserving writes.
- [Python reference](https://peter-gy.github.io/refkit/reference/python) records the public object and helper contracts.
- [Tidy options](https://peter-gy.github.io/refkit/reference/tidy-options) lists every formatting argument and default.
- [Pyodide](https://peter-gy.github.io/refkit/pyodide) records browser-hosted Python compatibility.

## License

RefKit is licensed under the
[Apache License 2.0](https://github.com/peter-gy/refkit/blob/main/LICENSE).
[NOTICE](https://github.com/peter-gy/refkit/blob/main/NOTICE) records upstream
citation and bibliography components.
