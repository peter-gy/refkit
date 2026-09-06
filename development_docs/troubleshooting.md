# Development Troubleshooting

Gather evidence at the failing boundary before rebuilding the full workspace.

## Python Tests Use An Older Native Module

Rebuild the Python adapter after changing `crates/refkit-core` or `packages/refkit/rust`:

```bash
(cd packages/refkit && uv run maturin develop)
```

Inspect `refkit.build_info` and `refkit.build_mode` when the loaded artifact remains unclear.

## Polars Cannot Load The Plugin

Rebuild the package-local native plugin:

```bash
(cd packages/polars-refkit && uv run maturin develop)
```

Keep Python Polars, Rust Polars, PyO3, and `pyo3-polars` within the tested application binary interface family. Update the package-local Cargo lockfile when that family changes.

## Import Reports A Version Mismatch

The `refkit` composition root compares package metadata with the native extension version. Remove mixed editable and installed copies from the active environment, then rebuild or install one complete release.

## Pyodide Rejects A Wheel

Check `.github/pyodide/runtime.json` first. It records the Python version, xbuild environment, PyEmscripten platform, Rust target, and tested Polars plugin family.

Run:

```bash
make pyodide-lock-check
```

A runtime claim requires a built PyEmscripten wheel and execution in the configured Pyodide environment.

## Coverage Fails After A Focused Test

The repository pytest command enforces full branch coverage across all Python packages. Use a test-path invocation with the repository coverage settings when validating the complete gate. Use `--no-cov` only for a bounded diagnostic run that is not completion evidence.

## The Docs Build Fails

Run the layers separately:

```bash
make docs-source-check
pnpm --dir docs typecheck
pnpm --dir docs build
```

The first command reports Markdown or audience-boundary problems. TypeScript reports configuration and theme errors. The static build verifier reports missing routes, assets, metadata, local links, and fragments.

## The Minimum Rust Check Installs A Toolchain

`make rust-floor` installs Rust 1.88 through rustup when the toolchain is absent. This command uses the network and changes the local rustup toolchain set. Install the toolchain explicitly before a disconnected validation run.

## Cleaning Invalidates Local Artifacts

`make clean` removes build trees, wheels, coverage and type caches, benchmark results, package bytecode, and native `.so` files under package directories. Rebuild both adapters before running Python tests after a full clean.
