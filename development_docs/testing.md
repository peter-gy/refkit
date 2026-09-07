# Testing

Test through the boundary that a consumer or maintainer depends on. Shared Rust behavior, Python conversion, Polars execution, installed wheels, and Pyodide each catch a different class of failure.

## Test Topology

| Boundary | Main location | Contract |
| --- | --- | --- |
| Portable Rust | `crates/refkit-core/src` and module tests | Parsing, recovery, raw edits, tidy formatting, rendering, and typed records. |
| Python adapter | `packages/refkit/tests` | Public objects, errors, helpers, stubs, native metadata, agent capability discovery, and packaged resources. |
| Polars | `packages/polars-refkit/tests` | Eager and lazy plans, namespace parity, broadcasting, dtypes, null mapping, diagnostics, and plugin packaging. |
| Benchmark tooling | `packages/refkit-bench/tests` | Lane selection, workload provenance, result shape, adapter correctness checks, and output files. |
| Repository contracts | `scripts/tests` | Architecture, documentation, versions, Pyodide locks, archive contents, and wheel normalization. |
| Built artifacts | `.github/workflows/ci.yml` and package artifact workflows | Wheel and sdist installation on supported runtimes and platforms. |
| Installed CPython and Pyodide runtime | `packages/refkit-tests/src/refkit_tests` | Public imports, parsing, raw edits, rendering, Polars callbacks, row failures, and installed agent examples. |
| Agent Plugin artifacts | Package tests, distribution contract, and installed smoke | Entry-point metadata, lazy imports, dynamic help, exact skill files, wheel and sdist markers, and Pyodide lookup. |
| Documentation site | `docs/`, `docs.yml`, and browser validation | TypeScript, root and Pages-base routes, links, assets, metadata, raw Markdown, llms indexes, search, themes, and responsive delivery. |

## Focused Commands

```bash
make test
make benchmark-test
make rust
make rust-floor
make docs-check
make build
```

`make test` runs the Python, Polars, benchmark, installed-runtime, and script contract suites with strict warnings and the configured coverage gate. `make rust` checks and tests the root and Polars Cargo workspaces. `make build` validates the contents of both Python distributions.

`make docs-examples-check` invokes `python -m refkit_tests.check_examples --root .` to execute authored README and guide examples against installed adapters in source integration. The packaged agent examples run through the installed artifact probes.

`make docs-check` installs the locked VitePress dependencies, runs TypeScript, builds root and `/refkit/` variants, and verifies the expected HTML, Markdown, llms, sitemap, and asset output. Browser inspection remains responsible for responsive layout, active navigation, search interaction, theme switching, and console or network failures.

## Run installed-artifact checks

`packages/refkit-tests` owns the shared probes, runtime tests, and installed agent-example runner. Root Ruff, ty, pyrefly, and pytest checks include this package.

Build its support wheel with:

```bash
uv build --package refkit-tests --wheel
```

Install that wheel alongside the candidate adapter wheel in a clean environment. Run from a directory outside the checkout so imports resolve through the installed artifacts:

```bash
python -m refkit_tests.smoke_refkit
python -m pytest --pyargs refkit_tests.test_refkit_runtime
```

For the Polars adapter:

```bash
python -m refkit_tests.smoke_polars_refkit
python -m pytest --pyargs refkit_tests.test_polars_refkit_runtime
```

Each probe selects its own adapter. The support package depends on the verification tools and leaves adapter installation to the caller. CI builds the support wheel once through `test-support.yml`, then installs it with the candidate packages in native, sdist, and Pyodide environments.

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
