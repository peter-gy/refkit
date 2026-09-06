# RefKit Repository Guide

RefKit exposes bibliography parsing, raw BibTeX editing, Citation Style Language rendering, and Polars expressions through one portable Rust capability core.

## Build And Validation

| Purpose | Command | Expected result |
| --- | --- | --- |
| Install | `uv sync --locked --all-packages --group dev` | Workspace packages resolve and install. |
| Build Python adapter | `make refkit-develop` | `refkit._native` and its Agent Plugin are installed. |
| Build Polars adapter | `(cd packages/polars-refkit && uv run maturin develop)` | `polars_refkit._internal` is installed. |
| Focused checks | `make lint typecheck test rust` | Source and boundary tests pass. |
| Complete gate | `make check` | Locks, contracts, tests, Rust floor, and distributions pass. |

Use the [developer documentation](development_docs/README.md) for architecture, workflows, tests, repository contracts, packaging, releases, and benchmarks.

## Architecture

- `crates/refkit-core` owns bibliography semantics. Its exported Rust types and operations form the portable core API used by every adapter.
- `packages/refkit` adapts the core to Python objects and composes the public `refkit` package with the native extension at `refkit._native`.
- `packages/polars-refkit` adapts the core to Polars expressions through its package-local Rust workspace.
- `packages/refkit-bench`, `scripts/`, and GitHub Actions are outward tooling. Runtime packages never depend on them.
- Hayagriva, BibLaTeX, and serializers are pure in-process implementation libraries. Add a trait port when the core must reach replaceable I/O or a runtime service.

## Dependency Rule

Dependencies point inward toward `crates/refkit-core`.

- The portable core accepts in-memory values and returns RefKit-owned structs, enums, errors, and result records.
- Filesystem paths, Python objects, PyO3, Polars values, plugin registration, and serialized host shapes belong to adapters.
- Hayagriva, BibLaTeX, and serializer types stay private when a RefKit type can express the contract.
- Rust adapters depend on the public `refkit-core` API and adapter support libraries. They never call Hayagriva or BibLaTeX directly.
- A new core dependency is an architecture decision. Classify it as a pure implementation library or place the reached capability behind a port.
- `scripts/architecture_contract.py` enforces allowed dependency sets, host-boundary ownership, workspace composition, released engine sources, and locked native builds.

Reject changes that bypass this graph. Extend the executable contract when source or artifact state can prove a new invariant.

## Ownership

| Path | Owns |
| --- | --- |
| `crates/refkit-core` | Parsing, recovery, normalized records, raw syntax, formatting, styles, rendering, and rendered trees. |
| `packages/refkit` | Filesystem access, PyO3 classes, exceptions, conversion, GIL boundaries, native registration, helpers, exports, stubs, and wheels. |
| `packages/polars-refkit` | Expression registration, dtypes, broadcasting, row failures, diagnostics, and plugin wheels. |
| `packages/refkit-bench` | Capability lanes, comparison adapters, fixtures, and result schemas. |
| `scripts` | Source, generated-state, release, and distribution contracts. |

## State Owners

- `Library` owns normalized citation data and parser diagnostics.
- `BibDocument` owns source-order raw BibTeX, occurrence identity, and edit-preserving writeback.
- `Document` owns prepared library, style, and locale inputs. Each render or bibliography call owns fresh citation state.
- The Python and Polars adapters own host conversion, registration, filesystem access, and runtime lifecycle.
- `refkit.agent` owns lazy code-mode guidance and version-matched Agent Plugin resource lookup. It calls the public Python API and defines no bibliography behavior.
- Release scripts own synchronized package versions and distribution metadata.

Do not create a second owner for one of these transitions.

## Testing Conventions

- Establish bibliography semantics in portable-core tests.
- Reuse bibliography inputs and expected semantic results across Python and Polars when both interfaces expose a capability.
- Keep Python conversion, GIL behavior, Polars broadcasting, dtype shape, null mapping, and plugin loading in adapter tests.
- Validate package behavior through installed wheels and Pyodide when editable imports cannot prove the contract.
- Treat generated locks and archives as consumed artifacts. Verify their source, generation command, freshness, and installation boundary.

Read the nearest scoped instructions before editing a package, crate, script, workflow, or developer document. Update the nearest `AGENTS.md` when a change creates a durable local invariant. Remove comments that narrate ordinary code before handoff.

## Focused Validation

| Change | Commands |
| --- | --- |
| Portable Rust core | Rebuild affected adapters, then `make rust-lint rust rust-floor` |
| Python adapter or public API | Run `make refkit-develop`, then `make python-lint typecheck test rust` |
| Polars expressions | Rebuild `packages/polars-refkit`, then `make typecheck test rust` |
| Docs or agent instructions | `make docs-check test` |
| Release or package metadata | `make release-check build` |
| GitHub Actions | `actionlint .github/workflows/*.yml` plus affected package checks |

Pyodide claims require built PyEmscripten wheels and runtime execution through `.github/pyodide`. Native artifacts require path remapping, wheel normalization, and distribution-contract validation.

Keep root `plugin.json`, `skills/refkit`, the `marimo.agent.capability` entry point, `refkit.agent`, and the Agent Plugins build configuration aligned. RefKit wheels and sdists must carry the exact curated skill inventory.

The user-facing documentation site lives under `docs/`. Keep its navigation exhaustive, preserve extensionless VitePress links, and run the pnpm typecheck, root and Pages-base builds, route, asset, llms, and link verifier through `make docs-check`. `pnpm --dir docs dev` serves `https://docs.refkit.localhost` through Portless. Use `pnpm --dir docs dev:direct` for the loopback VitePress server. `.github/workflows/pages.yml` deploys the `/refkit/` build.
