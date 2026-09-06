<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="docs/public/brand/refkit-lockup-horizontal-dark.svg">
    <img alt="RefKit" src="docs/public/brand/refkit-lockup-horizontal-light.svg" width="240">
  </picture>
</p>

<p align="center">
  Parse, render, inspect, format, and edit bibliography data from Python and Polars.
</p>

<p align="center">
  <a href="docs/index.md"><strong>Documentation</strong></a> ·
  <a href="packages/refkit/README.md"><strong>Python API</strong></a> ·
  <a href="packages/polars-refkit/README.md"><strong>Polars expressions</strong></a> ·
  <a href="development_docs/README.md"><strong>Contributing</strong></a>
</p>

<p align="center">
  <a href="https://github.com/peter-gy/refkit/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/peter-gy/refkit/actions/workflows/ci.yml/badge.svg"></a>
  <a href="https://pypi.org/project/refkit/"><img alt="refkit on PyPI" src="https://img.shields.io/pypi/v/refkit"></a>
  <a href="https://pypi.org/project/polars-refkit/"><img alt="polars-refkit on PyPI" src="https://img.shields.io/pypi/v/polars-refkit"></a>
  <img alt="Python 3.11 through 3.14" src="https://img.shields.io/badge/python-3.11%E2%80%933.14-3776ab">
  <a href="LICENSE"><img alt="Apache-2.0 license" src="https://img.shields.io/badge/license-Apache--2.0-0a0a0a"></a>
</p>

RefKit turns [BibTeX](https://ctan.org/pkg/bibtex), [BibLaTeX](https://ctan.org/pkg/biblatex), and [Hayagriva](https://github.com/typst/hayagriva) YAML into normalized bibliography records, applies [Citation Style Language](https://citationstyles.org/) styles, and returns plain text, HTML, or a structured render tree. Its raw BibTeX model preserves source-order blocks and duplicate occurrences for targeted field edits.

## Quickstart

Install the Python object interface:

```bash
python -m pip install refkit
```

Render one citation and its bibliography:

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

Expected output:

```text
(Doe, 2024)
Doe, J. (2024). Fast Citations. Journal of Citation Tests.
```

## What RefKit Owns

| User job | Interface |
| --- | --- |
| Parse, query, and project normalized bibliography entries | `Library` and `Entry` |
| Render ordered citations and cited or full bibliographies | `Style`, `Document`, `Citation`, and `Rendered` |
| Inspect and edit source-order raw BibTeX | `BibDocument`, `BibEntry`, and `BibField` |
| Canonically format BibTeX and report duplicate or missing-key warnings | `TidyOptions` and `TidyResult` |
| Apply parsing, rendering, inspection, and formatting inside Polars plans | `polars-refkit` and `pl.Expr.refkit` |
| Run the Python interfaces in a WebAssembly host | PyEmscripten wheels for Pyodide |

The portable `refkit-core` Rust crate owns bibliography semantics. Python owns paths, objects, and exceptions. Polars owns expressions, broadcasting, dtypes, and row failure mapping.

## Choose A Package

| Distribution | Import | Use it for |
| --- | --- | --- |
| `refkit` | `import refkit as rk` | Normalized libraries, citation documents, rendered trees, raw BibTeX edits, and path helpers. |
| `polars-refkit` | `import polars_refkit` | Bibliography source columns in eager and lazy Polars queries. |

Install the Polars interface with:

```bash
python -m pip install polars-refkit
```

A string argument names a Polars column. Wrap literal bibliography source or citation keys with `pl.lit(...)`.

## Learn RefKit

- [Get started](docs/get-started.md) reaches one complete render result.
- [How RefKit works](docs/concepts/how-refkit-works.md) defines the state owners and transitions.
- [Choose a bibliography model](docs/concepts/bibliography-models.md) distinguishes normalized data from source-preserving edits.
- [Python reference](docs/reference/python.md) lists the public object and helper contracts.
- [Polars reference](docs/reference/polars.md) lists all 21 expressions, dtypes, broadcasting, and failures.
- [Tidy options](docs/reference/tidy-options.md) defines every formatting argument and default.
- [Agent-readable documentation](docs/reference/agent-docs.md) links the generated page index, complete context file, and raw Markdown routes.

RefKit supports Python 3.11 through 3.14. The [Pyodide guide](docs/pyodide.md) records the tested Python, PyEmscripten, and Polars compatibility set.

## Contributing

The [developer documentation](development_docs/README.md) covers architecture, capability ownership, setup, testing, documentation delivery, packaging, releases, benchmarks, and troubleshooting.

## License

RefKit is licensed under the [Apache License 2.0](LICENSE). [NOTICE](NOTICE) records upstream citation and bibliography components.
