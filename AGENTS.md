# RefKit Repository Guide

## Setup

Prepare every workspace package and development tool:

```bash
uv sync --locked --all-packages --group dev
(cd packages/refkit-core && uv run maturin develop)
(cd packages/polars-refkit && uv run maturin develop)
```

Run the complete local gate with `make check`. It validates the release contract, Python and Rust formatting, lint, types, tests, the Rust 1.86 floor, and all three distributions.

## Ownership

| Path | Owns |
| --- | --- |
| `crates/refkit-core` | Platform-independent parsing, recovery, normalized records, raw BibTeX, formatting, styles, rendering, and rendered trees. |
| `packages/refkit-core` | PyO3 classes, exceptions, Python conversion, GIL boundaries, native wheels, and PyEmscripten wheels. |
| `packages/refkit` | Pure Python facade, exact `refkit-core` dependency, one-call helpers, and public stubs. |
| `packages/polars-refkit` | Polars expression registration, dtypes, broadcasting, row diagnostics, native plugin wheels, and PyEmscripten wheels. |
| `crates/bibtex-tidy-rs` | Rust compatibility surface over the formatter owned by `refkit-core`. |
| `packages/refkit-bench` | Capability benchmarks and competitor adapters. |

## Architecture Rules

- Keep `crates/refkit-core` independent of Python and Polars. Adapters depend on the core.
- Keep Python dictionaries, PyO3 objects, Polars dtypes, and serialized adapter shapes at the interface boundary.
- Keep `packages/polars-refkit/rust` as its own Cargo workspace. Its Polars and PyO3 versions form one ABI family.
- Update the root, `crates/bibtex-tidy-rs`, and `packages/polars-refkit/rust` Cargo lockfiles when a shared Rust dependency changes.
- Use released registry versions for BibLaTeX and Hayagriva. Inspect their local source checkouts before updating the dependency contract.
- Keep `refkit`, `refkit-core`, the root workspace, and `polars-refkit` on one release version. `scripts/release_contract.py` validates every repeated source.
- Update runtime exports, `.pyi` files, public docs, and boundary tests together when a Python API changes.

## Focused Validation

| Change | Commands |
| --- | --- |
| Python facade or stubs | `make python-lint typecheck test` |
| Portable Rust core | `make rust-lint rust rust-floor` |
| Native PyO3 adapter | `(cd packages/refkit-core && uv run maturin develop)` then `make typecheck test rust` |
| Polars expressions | `(cd packages/polars-refkit && uv run maturin develop)` then `make typecheck test rust` |
| Release metadata | `make release-check build` |
| GitHub Actions | `actionlint .github/workflows/*.yml` then the affected package tests |

Pyodide claims require a built PyEmscripten wheel and runtime execution through the smoke programs in `.github/pyodide`.

## Tests

- Assert behavior through `refkit`, `refkit_core`, Polars expressions, built packages, or rendered artifacts.
- Keep parser recovery, raw edit round trips, rendering history, row diagnostics, and package installation as explicit contracts.
- Keep fixture formatting intact when whitespace or malformed syntax is the test input.
- Add a regression at the narrowest public boundary that reproduces a fixed failure.

## CI And Release

`source-checks.yml` owns the reusable Python, Rust, and MSRV gates. `ci.yml` owns ordinary wheel and Pyodide artifact checks. `publish.yml` reuses both source checks and installed-artifact release tests before publishing each package in dependency order.

Pin third-party actions to commit SHAs. Keep package build jobs separate from installed-package smoke jobs so failures identify the affected boundary.

Native release jobs must configure Rust path remapping before compilation, normalize generated wheel SBOMs, and run `scripts/distribution_contract.py` before upload. The contract rejects bytecode, local SBOM references, and builder paths embedded in archive contents.
