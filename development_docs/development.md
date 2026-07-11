# Development Workflow

Use package-focused rebuilds and checks while iterating. Run the complete repository gate before handoff.

## Bootstrap

```bash
uv sync --locked --all-packages --group dev
(cd packages/refkit-core && uv run maturin develop)
(cd packages/polars-refkit && uv run maturin develop)
```

`refkit-core` and `polars-refkit` contain native modules. Rebuild the affected module after Rust changes or Python tests can exercise an older editable binary.

## Iterate By Boundary

| Change | Rebuild | Focused checks |
| --- | --- | --- |
| Pure Python facade or stubs | None | `make python-lint typecheck test` |
| Portable Rust core | Rebuild each affected Python adapter | `make rust-lint rust rust-floor` |
| Native PyO3 adapter | `(cd packages/refkit-core && uv run maturin develop)` | `make typecheck test rust` |
| Polars expressions or plugin Rust | `(cd packages/polars-refkit && uv run maturin develop)` | `make typecheck test rust` |
| Benchmark runner | Build both adapters in release mode | `make benchmark-test` |
| Documentation | None | `make docs-check` |
| Release or package metadata | Build affected distributions | `make release-check build` |
| GitHub Actions | None | `actionlint .github/workflows/*.yml` plus affected package checks |

Use `make format` to apply Ruff and Rust formatting across all workspaces. `make lint` checks Python and Rust formatting and lint rules without rewriting files.

## Change A Public Python API

1. Place platform-independent behavior and records in `crates/refkit-core`.
2. Expose Python classes or conversions from `packages/refkit-core/rust` when the native boundary changes.
3. Update `packages/refkit-core/src/refkit_core/_refkit_core.pyi` and package exports.
4. Update the facade runtime and `packages/refkit/src/refkit/__init__.pyi` when the public `refkit` surface changes.
5. Update Polars namespace methods, top-level functions, runtime signatures, stubs, and Rust keyword records together when the capability is exposed as an expression.
6. Add tests through the nearest public boundary and update end-user API documentation.

Keep Python dictionaries, Polars structs, JSON output, and exception mapping in adapters. Core records should describe bibliography behavior in Rust types.

## Change Rust Dependencies

The repository contains three Cargo workspaces:

- root `Cargo.lock` for the portable core and native Python adapter
- `crates/bibtex-tidy-rs/Cargo.lock` for the compatibility crate
- `packages/polars-refkit/rust/Cargo.lock` for the Polars plugin ABI family

Update every lockfile whose manifest resolves the changed dependency. Keep Polars, `polars-core`, PyO3, and `pyo3-polars` aligned inside the package-local workspace. Run `make rust rust-floor` after resolution changes.

RefKit consumes released Hayagriva and BibLaTeX crates. Inspect the resolved registry source and upstream release notes before changing their feature or version contract.

## Record Durable Knowledge

Put a short invariant in the nearest `AGENTS.md` when it applies to one package or module. Put cross-package reasoning and maintenance procedures in `development_docs/`. Add an executable repository check when source or artifact state can prove the rule.

Session notes, temporary experiments, build products, and local benchmark results stay outside the tracked project surface.

## Complete The Change

```bash
make check
```

The full gate is intentionally singular. It catches version drift, architecture violations, documentation links, stale locks, type errors, cross-language test failures, minimum Rust version failures, and invalid distribution contents.
