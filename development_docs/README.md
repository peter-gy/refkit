# RefKit Developer Documentation

These documents describe how to change, test, package, and release RefKit. The root README, `docs/`, and published package READMEs own installation and public API guidance.

## Set Up The Workspace

```bash
uv sync --locked --all-packages --group dev
(cd packages/refkit-core && uv run maturin develop)
(cd packages/polars-refkit && uv run maturin develop)
```

Run `make check` before handing off a repository change. It validates locks, release metadata, architecture, documentation, Pyodide inputs, Python and Rust code, tests, the Rust 1.88 floor, and built distributions.

## Find The Right Document

| Task | Read |
| --- | --- |
| Place behavior in the correct crate or package | [Architecture](architecture.md) |
| Set up an edit and rebuild loop | [Development workflow](development.md) |
| Choose the test boundary and completion gate | [Testing](testing.md) |
| Update locks, stubs, generated state, or repository policy | [Repository contracts](repository-contracts.md) |
| Build wheels, validate Pyodide, or prepare a release | [Packaging and release](packaging-and-release.md) |
| Measure a capability or interpret benchmark output | [Benchmarks](benchmarks.md) |
| Compare RefKit coverage with inspected reference packages | [Feature matrix](feature-matrix.md) |

The nearest `AGENTS.md` owns short, local invariants. Put explanations and cross-package workflows in this directory. When an invariant can be checked from source or an artifact, extend the corresponding script under `scripts/` and keep it in `make check`.

## Documentation Boundary

End-user documentation covers installation, supported inputs, public APIs, return values, errors, migration paths, and runtime use. Developer documentation covers source ownership, dependency direction, build tools, test topology, benchmarks, CI, packaging, and release procedures.

`make docs-check` verifies that this directory contains Markdown files, local links resolve, and public docs stay independent from developer pages. `scripts/distribution_contract.py` verifies that built distributions exclude this directory.
