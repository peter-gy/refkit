# Testing

Test through the boundary that a consumer or maintainer depends on. Shared Rust behavior, Python conversion, Polars execution, installed wheels, and Pyodide each catch a different class of failure.

## Test Topology

| Boundary | Main location | Contract |
| --- | --- | --- |
| Portable Rust | `crates/refkit-core/src` and module tests | Parsing, recovery, raw edits, tidy formatting, rendering, and typed records. |
| Python adapter | `packages/refkit/tests` | Public objects, errors, helpers, stubs, native metadata, and composition under a mocked Pyodide import. |
| Polars | `packages/polars-refkit/tests` | Eager and lazy plans, namespace parity, broadcasting, dtypes, null mapping, diagnostics, and plugin packaging. |
| Benchmark tooling | `packages/refkit-bench/tests` | Lane selection, workload provenance, result shape, adapter correctness checks, and output files. |
| Repository contracts | `scripts/tests` | Architecture, documentation, versions, Pyodide locks, archive contents, and wheel normalization. |
| Built artifacts | `.github/workflows/ci.yml` and release-test workflows | Wheel and sdist installation on supported runtimes and platforms. |
| Pyodide runtime | `.github/pyodide` | Public imports, parsing, raw edits, rendering, Polars callbacks, and row failure behavior. |
| Documentation site | `docs/`, `pages.yml`, and browser validation | TypeScript, root and Pages-base routes, links, assets, metadata, raw Markdown, llms indexes, search, themes, and responsive delivery. |

## Focused Commands

```bash
make test
make benchmark-test
make rust
make rust-floor
make docs-check
make build
```

`make test` runs the Python, Polars, benchmark, and script contract suites with strict warnings and full branch coverage for the Python packages. `make rust` checks and tests the root and Polars Cargo workspaces. `make build` validates the contents of both Python distributions.

`make docs-check` installs the locked VitePress dependencies, runs TypeScript, builds root and `/refkit/` variants, and verifies the expected HTML, Markdown, llms, sitemap, and asset output. Browser inspection remains responsible for responsive layout, active navigation, search interaction, theme switching, and console or network failures.

## Choose The Assertion Boundary

- Parser and raw edit regressions should use the smallest input that preserves the failing syntax.
- Python behavior should be asserted through `refkit` objects that users import.
- Polars behavior should be asserted through expressions in eager and lazy plans where execution mode is relevant.
- Package regressions should install the built wheel or sdist when editable imports could hide the failure.
- Pyodide claims require a PyEmscripten wheel and execution inside the configured Pyodide runtime.
- Browser embedding claims require a `loadPyodide` and `micropip` run in a real browser or Node.js host. The current release workflows validate the Pyodide command-line runtime.
- Rendering changes should cover affected text, HTML, tree, ordered citations within one call, sorting, and bibliography boundaries.

Preserve whitespace and malformed syntax in fixtures when those bytes are part of the behavior. Assert concrete public values instead of helper output, internal iteration order, generated formatting, or implementation names.

## Cross-Interface Semantics

The interfaces expose different call shapes, so one identical test suite would obscure host-specific contracts. Reuse the same bibliography inputs and expected semantic results where a capability crosses boundaries.

Examples include:

- normalized entry counts and keys in Rust, Python, and Polars
- citation text from `Document.render` and Polars citation expressions
- diagnostics from recoverable and strict parsing
- tidy output and warning records
- installed-wheel imports on CPython and Pyodide

Host-specific tests remain responsible for conversion and lifecycle behavior such as Python exceptions, GIL release, Polars nulls, dtype shape, expression broadcasting, and plugin loading.

## Completion Gate

Run `make check` after focused checks pass. It is the local completion gate for source, current-host tests, and current-host package archives. CI adds clean installed-wheel tests, platform matrices, and Pyodide runtime execution.
