# RefKit Developer Documentation

These documents describe how to change, test, document, package, and release RefKit. The root README, `docs/`, and published package READMEs own installation and public API guidance.

## Prerequisites

- [uv](https://docs.astral.sh/uv/) resolves the Python workspace and installs its development groups.
- [Rust and Cargo](https://www.rust-lang.org/tools/install) compile and test the portable core, native Python module, and Polars plugin.
- [Node.js 24 or newer](https://nodejs.org/) runs Portless, VitePress, and the documentation verifier.
- [pnpm](https://pnpm.io/installation) installs the locked documentation dependency graph.

Workspace synchronization installs [Maturin](https://www.maturin.rs/), the builder used to compile Rust-backed Python extensions.

## Set Up The Workspace

```bash
make sync
make refkit-develop
make polars-refkit-develop
```

Run `make check` before handing off a repository change. It validates locks, release metadata, architecture, documentation, Pyodide inputs, Python and Rust code, tests, the Rust 1.88 floor, and built distributions.

## Find The Right Document

| Task | Read |
| --- | --- |
| Place behavior in the correct crate or package | [Architecture](architecture.md) |
| Trace a capability across owners, interfaces, and tests | [Capability map](capabilities.md) |
| Preserve parser, raw edit, tidy, and render invariants | [Core semantics](core-semantics.md) |
| Change Python, Polars, or Pyodide host boundaries | [Adapter contracts](adapters.md) |
| Set up an edit and rebuild loop | [Development workflow](development.md) |
| Choose the test boundary, installed `refkit-tests` probes, and completion gate | [Testing](testing.md) |
| Build and validate the VitePress site | [Documentation site](documentation.md) |
| Update locks, stubs, generated state, or repository policy | [Repository contracts](repository-contracts.md) |
| Build wheels, validate Pyodide, or prepare a release | [Packaging and release](packaging-and-release.md) |
| Measure a capability or interpret benchmark output | [Benchmarks](benchmarks.md) |
| Choose an equivalent reference-package workflow | [Bibliography package boundaries](feature-matrix.md) |
| Diagnose local build, plugin, runtime, or docs failures | [Development troubleshooting](troubleshooting.md) |

The nearest `AGENTS.md` owns short, local invariants. Put explanations and cross-package workflows in this directory. When an invariant can be checked from source or an artifact, extend the corresponding script under `scripts/` and keep it in `make check`.

## Documentation Boundary

End-user documentation covers installation, supported inputs, public APIs, return values, errors, and runtime use. Developer documentation covers source ownership, dependency direction, build tools, test topology, benchmarks, CI, packaging, and release procedures.

`make docs-check` validates Markdown boundaries, installs the locked VitePress dependencies, typechecks and builds the site, then verifies routes, public assets, metadata, and local links. `scripts/distribution_contract.py` verifies that built distributions exclude this directory.
