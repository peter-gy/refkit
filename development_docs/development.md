# Development Workflow

Use package-focused rebuilds and checks while iterating. Run the complete repository gate before handoff.

## Bootstrap

Install the uv, Rust, Node.js, and pnpm prerequisites from the [developer index](README.md), then synchronize the Python workspace. The development group provides Maturin for native adapter builds.

```bash
uv sync --locked --all-packages --group dev
make refkit-develop
(cd packages/polars-refkit && uv run maturin develop)
```

`refkit` and `polars-refkit` contain native modules. Rebuild the affected module after Rust changes or Python tests can exercise an older editable binary.

## Iterate By Boundary

| Change | Rebuild | Focused checks |
| --- | --- | --- |
| Pure Python facade or stubs | None | `make python-lint typecheck test` |
| Portable Rust core | Rebuild each affected Python adapter | `make rust-lint rust rust-floor` |
| Native PyO3 adapter or Agent Plugin | `make refkit-develop` | `make typecheck test rust` |
| Polars expressions or plugin Rust | `(cd packages/polars-refkit && uv run maturin develop)` | `make typecheck test rust` |
| Benchmark runner | Build both adapters in release mode | `make benchmark-test` |
| User documentation or site | Install locked pnpm dependencies | `make docs-check test` |
| Release or package metadata | Build affected distributions | `make release-check build` |
| GitHub Actions | None | `actionlint .github/workflows/*.yml` plus affected package checks |

Use `make format` to apply Ruff and Rust formatting across all workspaces. `make lint` checks Python and Rust formatting and lint rules without rewriting files.

## Change A Public Python API

1. Place platform-independent behavior and records in `crates/refkit-core`.
2. Expose Python classes or conversions from `packages/refkit/rust` when the native boundary changes.
3. Update `packages/refkit/src/refkit/_native.pyi`, package exports, and `__init__.pyi`.
4. Update Polars namespace methods, top-level functions, runtime signatures, stubs, and Rust keyword records together when the capability is exposed as an expression.
5. Add tests through the nearest public boundary and update end-user API documentation.

Keep Python dictionaries, Polars structs, JSON output, and exception mapping in adapters. Core records should describe bibliography behavior in Rust types.

## Add A Capability

1. Define the user behavior, input, result, failure modes, and state owner.
2. Add portable behavior and RefKit-owned records to `crates/refkit-core`.
3. Test the semantic contract through the core API.
4. Project the result through each intended adapter using host-native values.
5. Add adapter tests for conversion, lifecycle, and failure behavior.
6. Update the public guides, exact reference, stubs, and the [capability map](capabilities.md).
7. Extend an executable repository contract when source or artifact state can prove the new invariant.

Keep one owner for each state transition. An adapter converts values and manages its host runtime. It should not reimplement bibliography semantics.

## Add A Polars Expression

Treat one expression as a cross-language surface. Update these parts together:

1. Top-level builder in `polars_refkit/_expressions.py`.
2. `pl.Expr.refkit` namespace method in `_namespace.py`.
3. Runtime exports, `__all__`, and `__init__.pyi`.
4. Static option conversion or serde keyword records.
5. Rust expression registration and core call.
6. Exact scalar, list, or struct dtype constructor.
7. Default output name, null propagation, broadcasting, and lazy behavior.
8. Eager, lazy, dtype, row-failure, and installed-wheel tests.
9. Polars guide and expression reference.

Static Python validation happens while constructing the expression. Dtype checks, style loading, projection validation, broadcasting, and row execution happen when an eager query runs or a lazy plan collects.

## Change The Documentation Site

Keep `docs/.vitepress/config.mts`, the public page tree, and the static build verifier aligned. Add every public page to the sidebar unless it is an intentional compatibility route.

Run:

```bash
make docs-check test
```

Use the [documentation site guide](documentation.md) for browser and delivery validation.

## Change Rust Dependencies

The repository contains two Cargo workspaces:

- root `Cargo.lock` for the portable core and native Python adapter
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
