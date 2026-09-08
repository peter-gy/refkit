<p align="center">
  <a href="https://peter-gy.github.io/refkit/">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="docs/public/brand/refkit-lockup-horizontal-dark-transparent.svg">
      <source media="(prefers-color-scheme: light)" srcset="docs/public/brand/refkit-lockup-horizontal-light-transparent.svg">
      <img alt="RefKit" src="docs/public/brand/refkit-lockup-horizontal-light-transparent.svg" width="280">
    </picture>
  </a>
</p>

<p align="center">
  Parse, cite, and edit bibliography data from Python and Polars.
</p>

<p align="center">
  <a href="https://pypi.org/project/refkit/"><img src="https://img.shields.io/pypi/v/refkit" alt="PyPI version"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-blue" alt="License: Apache-2.0"></a>
  <a href="https://github.com/peter-gy/refkit/actions/workflows/ci.yml?query=branch%3Amain"><img src="https://github.com/peter-gy/refkit/actions/workflows/ci.yml/badge.svg?branch=main&amp;event=push" alt="CI on main"></a>
</p>

<p align="center">
  <a href="https://peter-gy.github.io/refkit/">Documentation</a> ·
  <a href="https://peter-gy.github.io/refkit/get-started">Quickstart</a> ·
  <a href="https://peter-gy.github.io/refkit/reference/agent-docs">For agents</a> ·
  <a href="development_docs/README.md">Development</a>
</p>

RefKit reads [BibTeX](https://ctan.org/pkg/bibtex),
[BibLaTeX](https://ctan.org/pkg/biblatex), and
[Hayagriva YAML](https://github.com/typst/hayagriva), then turns your references
into citations, bibliographies, and inspectable records. Its raw BibTeX model
supports targeted field edits.

- **Render citations and bibliographies** with bundled or custom [Citation Style Language](https://citationstyles.org/) styles. Get text, HTML, or a structured tree.
- **Inspect and repair bibliography data.** Select entries, project fields, review parsing diagnostics, and edit existing BibTeX fields while preserving surrounding source text.
- **Format and deduplicate BibTeX.** Configure layout, generate citation keys, and review warnings before accepting changes.
- **Work inside [Polars](https://pola.rs/) queries.** Apply bibliography operations to columns through `pl.Expr.refkit` in eager or lazy dataframes.

## Render a citation

Install in Python 3.11 through 3.14:

```bash
python -m pip install refkit
```

```python
import refkit as rk

library = rk.Library.parse_bibtex("""
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  journal = {Journal of Citation Tests},
  year = {2024}
}
""")
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
result = document.render([rk.Citation("intro", "doe2024")])

print(result["intro"].text)
print(result.bibliography.text)
```

```text
(Doe, 2024)
Doe, J. (2024). Fast Citations. Journal of Citation Tests.
```

Pass related citations in one ordered render call so numbering and
disambiguation share the same context. Continue with the
[Python quickstart](https://peter-gy.github.io/refkit/get-started) or install
`polars-refkit` for the [Polars guide](https://peter-gy.github.io/refkit/guides/polars).

## Use with agents

The `refkit` package includes an [Agent Plugin](https://agent-plugins.org/)
with version-matched task guidance. An agent that can execute Python can
discover the installed workflow:

```python
import refkit.agent

help(refkit.agent)
```

[Agent integration](https://peter-gy.github.io/refkit/reference/agent-docs)
covers discovery and packaged resources.

## Explore

[Parse and recover](https://peter-gy.github.io/refkit/guides/parse-bibliographies) ·
[Edit BibTeX](https://peter-gy.github.io/refkit/guides/edit-bibtex) ·
[Format BibTeX](https://peter-gy.github.io/refkit/guides/format-bibtex) ·
[Python reference](https://peter-gy.github.io/refkit/reference/python) ·
[Pyodide](https://peter-gy.github.io/refkit/pyodide)

RefKit is alpha software. Public APIs may change before 1.0.

## Acknowledgements

RefKit uses [Typst](https://github.com/typst/)'s
[Hayagriva](https://github.com/typst/hayagriva) for bibliography modeling and citation rendering,
and [BibLaTeX](https://github.com/typst/biblatex) for parsing.
Its BibTeX formatter follows [bibtex-tidy](https://github.com/FlamingTempura/bibtex-tidy),
whose specification tests also inform RefKit's formatter checks and benchmarks.

[Development](development_docs/README.md) ·
[Report an issue](https://github.com/peter-gy/refkit/issues) ·
[Apache-2.0 license](LICENSE) · [Upstream notices](NOTICE)
