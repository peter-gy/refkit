<p align="center">
  <a href="https://peter-gy.github.io/refkit/">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="docs/public/brand/refkit-lockup-horizontal-dark-transparent.svg">
      <source media="(prefers-color-scheme: light)" srcset="docs/public/brand/refkit-lockup-horizontal-light-transparent.svg">
      <img alt="RefKit" src="docs/public/brand/refkit-lockup-horizontal-light-transparent.svg" width="420">
    </picture>
  </a>
</p>

<p align="center">
  Parse, cite, tidy, and edit bibliography data from Python and Polars.
</p>

<p align="center">
  <a href="https://peter-gy.github.io/refkit/"><strong>Documentation</strong></a> ·
  <a href="packages/refkit/README.md"><strong>Python API</strong></a> ·
  <a href="packages/polars-refkit/README.md"><strong>Polars expressions</strong></a> ·
  <a href="development_docs/README.md"><strong>Development</strong></a>
</p>

<p align="center">
  <a href="https://pypi.org/project/refkit/"><img alt="refkit on PyPI" src="https://img.shields.io/pypi/v/refkit.svg"></a>
  <a href="https://pypi.org/project/polars-refkit/"><img alt="polars-refkit on PyPI" src="https://img.shields.io/pypi/v/polars-refkit.svg"></a>
  <a href="https://github.com/peter-gy/refkit/actions/workflows/ci.yml"><img alt="CI status" src="https://github.com/peter-gy/refkit/actions/workflows/ci.yml/badge.svg?branch=main"></a>
  <a href="https://pypi.org/project/refkit/"><img alt="Supported Python versions" src="https://img.shields.io/pypi/pyversions/refkit.svg"></a>
  <a href="LICENSE"><img alt="Apache-2.0 license" src="https://img.shields.io/pypi/l/refkit.svg"></a>
</p>

> [!NOTE]
> RefKit is alpha software. Public APIs may change before 1.0.

RefKit turns [BibTeX](https://ctan.org/pkg/bibtex),
[BibLaTeX](https://ctan.org/pkg/biblatex), and
[Hayagriva](https://github.com/typst/hayagriva) YAML into normalized
bibliography records. It renders citations and bibliographies with
[Citation Style Language](https://citationstyles.org/) styles, produces plain
text, HTML, or structured trees, and preserves raw BibTeX for targeted edits.
The `polars-refkit` package exposes the same parsing, rendering, inspection,
and formatting capabilities as expressions for [Polars](https://pola.rs/)
eager and lazy queries.

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
rendered = document.render([rk.Citation("intro", "doe2024")])

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

`TidyOptions` configures canonical BibTeX formatting and duplicate handling
for `tidy_bibtex()` and `BibDocument.tidy()`.

## Work with bibliography data

| Task | Interface | Start here |
| --- | --- | --- |
| Parse, render, inspect, and edit from Python | `refkit` | [Python quickstart](https://peter-gy.github.io/refkit/get-started) |
| Apply bibliography operations to eager or lazy dataframes | `polars-refkit` and `pl.Expr.refkit` | [Polars guide](https://peter-gy.github.io/refkit/guides/polars) |
| Run the Python interfaces in a browser-hosted Python runtime | WebAssembly wheels for [Pyodide](https://pyodide.org/) | [Pyodide guide](https://peter-gy.github.io/refkit/pyodide) |
| Give a code-mode agent a version-matched RefKit workflow | `refkit.agent` and its packaged [Agent Skill](https://agentskills.io/specification) | [Agent integration](https://peter-gy.github.io/refkit/reference/agent-docs) |

A plain string passed to a Polars expression names a column. Wrap literal
bibliography source or citation keys with `pl.lit(...)`.

## Use with code-mode agents

The `refkit` wheel contains the Python library, an
[Agent Plugin](https://agent-plugins.org/), and a version-matched
[Agent Skill](https://agentskills.io/specification). Installing `refkit` places
the library and agent resources in the same Python environment. A code-mode
agent that can execute Python can inspect the dynamic module documentation:

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
can use these imports directly. [Marimo](https://marimo.io/), for example,
discovers `refkit.agent` automatically through the capability entry point
installed with the wheel.

## Documentation

- [Get started](https://peter-gy.github.io/refkit/get-started) reaches a complete render result.
- [How RefKit works](https://peter-gy.github.io/refkit/concepts/how-refkit-works) defines the state owners and transitions.
- [Guides](https://peter-gy.github.io/refkit/guides/parse-bibliographies) cover Python, Polars, raw BibTeX editing, and custom Citation Style Language files.
- [Reference](https://peter-gy.github.io/refkit/reference/python) records the Python, Polars, Rust, data-shape, tidy, error, and agent contracts.
- [Performance](https://peter-gy.github.io/refkit/performance) publishes reproducible benchmark evidence.
- [Troubleshooting](https://peter-gy.github.io/refkit/troubleshooting) maps common failures to recovery steps.

## Development

The [development documentation](development_docs/README.md) covers setup,
architecture, testing, documentation delivery, packaging, releases, and
benchmarks. Report defects through
[GitHub Issues](https://github.com/peter-gy/refkit/issues).

## License

RefKit is licensed under the [Apache License 2.0](LICENSE). [NOTICE](NOTICE)
records upstream citation and bibliography components.
