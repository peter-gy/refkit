# Testing

Test through the boundary that a consumer or maintainer depends on. Shared Rust behavior, Python conversion, Polars execution, installed wheels, and Pyodide each catch a different class of failure.

## Test Topology

| Boundary | Main location | Contract |
| --- | --- | --- |
| Portable Rust | `crates/refkit-core/src` and module tests | Parsing, recovery, raw edits, tidy formatting, rendering, and typed records. |
| Native and facade Python | `packages/refkit/tests` | Public objects, errors, helpers, stubs, runtime version checks, and Pyodide metadata. |
| Polars | `packages/polars-refkit/tests` | Eager and lazy plans, namespace parity, broadcasting, dtypes, null mapping, diagnostics, and plugin packaging. |
| Benchmark tooling | `packages/refkit-bench/tests` | Lane selection, workload provenance, result shape, adapter correctness checks, and output files. |
| Repository contracts | `scripts/tests` | Architecture, documentation, versions, Pyodide locks, archive contents, and wheel normalization. |
| Built artifacts | `.github/workflows/ci.yml` and release-test workflows | Wheel and sdist installation on supported runtimes and platforms. |
| Pyodide runtime | `.github/pyodide` | Public imports, parsing, raw edits, rendering, Polars callbacks, and row failure behavior. |

## Focused Commands

```bash
make test
make benchmark-test
make rust
make rust-floor
make docs-check
make build
```

`make test` runs the Python, Polars, benchmark, and script contract suites with strict warnings and full branch coverage for the Python packages. `make rust` checks and tests the root, compatibility, and Polars Cargo workspaces. `make build` validates the contents of all local Python distributions.

## Choose The Assertion Boundary

- Parser and raw edit regressions should use the smallest input that preserves the failing syntax.
- Python behavior should be asserted through `refkit` or `refkit_core` objects that users import.
- Polars behavior should be asserted through expressions in eager and lazy plans where execution mode is relevant.
- Package regressions should install the built wheel or sdist when editable imports could hide the failure.
- Pyodide claims require a PyEmscripten wheel and execution inside the configured Pyodide runtime.
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
