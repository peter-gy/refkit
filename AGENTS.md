# RefKit Repository Guide

RefKit unifies bibliography parsing, raw BibTeX editing, CSL rendering, and Polars expressions through one portable Rust capability core. Keep semantics in the core and host conversion at adapter boundaries.

## Start Here

```bash
uv sync --locked --all-packages --group dev
(cd packages/refkit-core && uv run maturin develop)
(cd packages/polars-refkit && uv run maturin develop)
```

Run `make check` before handoff. Use the [developer documentation](development_docs/README.md) for architecture, workflows, tests, repository contracts, packaging, releases, and benchmarks.

## Ownership

| Path | Owns |
| --- | --- |
| `crates/refkit-core` | Portable parsing, recovery, normalized records, raw BibTeX, formatting, styles, rendering, and rendered trees. |
| `packages/refkit-core` | PyO3 classes, exceptions, conversion, GIL boundaries, and native wheels. |
| `packages/refkit` | Pure Python facade, exact core dependency, helpers, exports, and public stubs. |
| `packages/polars-refkit` | Polars registration, dtypes, broadcasting, row failures, diagnostics, and plugin wheels. |
| `crates/bibtex-tidy-rs` | Rust formatter compatibility surface over the shared core. |
| `packages/refkit-bench` | Capability lanes, comparison adapters, fixtures, and result schemas. |

## Invariants

- Adapters depend on `crates/refkit-core`. Keep Python, PyO3, Polars, and serialized host shapes out of the portable core.
- `Library` owns normalized citation data. `BibDocument` owns source-order raw BibTeX and edit preservation.
- Keep the Polars Rust package as its own Cargo workspace and align its Polars, PyO3, and `pyo3-polars` ABI family.
- Keep the publishable runtime release set synchronized across Python and Rust. Keep `refkit` pinned to the exact matching `refkit-core`. `refkit-bench` has its own version.
- Update runtime exports, `.pyi` files, public docs, and boundary tests together for public API changes.
- Keep end-user material in root or package READMEs and `docs/`. Keep maintainer and agent detail in `development_docs/`.
- Extend an executable contract under `scripts/` when repository or artifact state can prove an invariant.

## Focused Validation

| Change | Commands |
| --- | --- |
| Python facade or stubs | `make python-lint typecheck test` |
| Portable Rust core | `make rust-lint rust rust-floor` |
| Native PyO3 adapter | Rebuild `packages/refkit-core`, then `make typecheck test rust` |
| Polars expressions | Rebuild `packages/polars-refkit`, then `make typecheck test rust` |
| Docs or agent instructions | `make docs-check test` |
| Release or package metadata | `make release-check build` |
| GitHub Actions | `actionlint .github/workflows/*.yml` plus affected package checks |

Pyodide claims require built PyEmscripten wheels and runtime execution through `.github/pyodide`. Native artifacts require path remapping, wheel normalization, and distribution-contract validation.

Read the nearest scoped instructions before editing the portable core, native adapter, Python facade, Polars adapter, compatibility crate, benchmark package, repository scripts, or GitHub Actions.

Record a durable local invariant in the nearest `AGENTS.md`. Put cross-package reasoning in `development_docs/` and keep session state out of tracked guidance.
