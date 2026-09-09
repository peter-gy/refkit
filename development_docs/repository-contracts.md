# Repository Contracts

RefKit keeps cross-file invariants executable. Each contract has one source-level entry point, focused tests, a Make target, and a place in the complete gate.

## Contract Scripts

| Contract | Command | Protects |
| --- | --- | --- |
| Architecture | `make architecture-check` | Core dependency classification, host-boundary ownership, workspace composition, audited engine sources, adapter direction, and locked native builds. |
| Documentation source | `make docs-source-check` | Markdown-only developer docs, local link targets, VitePress routes, and the public-to-developer audience boundary. |
| Documentation site | `make docs-site-check` | Locked pnpm install, TypeScript, root and Pages-base VitePress output, routes, public assets, social metadata, raw Markdown, llms indexes, local links, and heading fragments. |
| JavaScript package | `make js-check` | Source freshness, npm exports, installed TypeScript consumer, Python parity, automatic memory lifetime, and browser and worker execution. |
| Release metadata | `make release-check` | Lockstep versions, exact native dependency pins, repository metadata, and release tag grammar. |
| Pyodide runtime | `make pyodide-lock-check` | Runtime requirements, resolved wheels, hashes, and the tested Python-to-Rust Polars plugin ABI mapping. |
| Distribution archive | `scripts/distribution_contract.py <archives>` | Bytecode exclusion, developer-doc exclusion, builder-path removal, and exact package-specific Agent Plugin resources. |

Contract diagnostics should name the offending source or archive member and return a nonzero exit status. Keep validation deterministic and free from network access. Test a new failure mode beside the script before adding it to `make check` or CI.

## Files That Change Together

### Python API

- native PyO3 definitions under `packages/refkit/rust/src`
- native stubs in `packages/refkit/src/refkit/_native.pyi`
- public exports, stubs, and helpers in `packages/refkit/src/refkit`
- public tests and end-user API docs

### JavaScript API

- native WebAssembly adapter under `packages/refkit-js/rust`
- public TypeScript exports, runtime classes, and types under `packages/refkit-js/src`
- package exports and lockfile under `packages/refkit-js`
- installed-package tests, browser tests, and Python parity comparisons

### Agent capability

- root `plugin.json` and `skills/refkit`, plus the Polars package plugin and skill tree
- `packages/refkit/src/refkit/agent.py` and `packages/polars-refkit/polars_refkit/agent.py`
- the `marimo.agent.capability` entry point and package runtime dependency
- package-local Agent Plugins build backend
- direct-Maturin wheel augmentation
- exact source-plan, wheel, sdist, installed-resource, and dynamic-help tests

### Polars API

- top-level functions in `packages/polars-refkit/polars_refkit/_expressions.py`
- namespace methods in `_namespace.py`
- public exports and `__init__.pyi`
- Rust expression registration and keyword records
- eager, lazy, dtype, broadcast, null, and installed-wheel tests

### Release version

- root Cargo workspace version and shared Rust dependency
- root Python workspace version
- `refkit` and `polars-refkit` project versions
- native adapter and Polars Rust crate versions
- JavaScript package, npm lockfile, and WebAssembly adapter versions

`scripts/release_contract.py` lists the authoritative repeated sources.

### Rust dependencies

- root `Cargo.lock`
- `packages/polars-refkit/rust/Cargo.lock`
- `packages/refkit-js/rust/Cargo.lock`

Update each lockfile whose workspace resolves the dependency. The Polars workspace keeps its plugin ABI family local.

### Pyodide runtime

- `.github/pyodide/runtime.json`
- `.github/pyodide/requirements.in`
- `.github/pyodide/pylock.314.toml`
- `docs/pyodide.md` and public package support statements
- setup actions, build matrices, and smoke programs when the runtime contract changes

Run `make pyodide-lock` to regenerate the lock, then `make pyodide-lock-check`.

## Tracked And Derived Artifacts

| Artifact | Policy |
| --- | --- |
| Python `.pyi` files | Tracked public contracts. Update them with runtime exports. |
| Cargo and uv locks | Tracked resolution contracts. Run `make lock-upgrade` to refresh uv from `pyproject.toml`, then validate the locks before tests. |
| Pyodide lock | Tracked generated runtime input. Regenerate through `scripts/pyodide_lock.py`. |
| Wheels and sdists | Derived build output. Validate archives and leave them untracked. |
| Installed-test support wheel | Built from `packages/refkit-tests` and installed alongside candidate adapters. Run its probes outside the checkout. |
| Agent Plugin wheel payload | Derived from each package's configured plugin manifest and skill tree. Validate its exact inventory in wheels and sdists. |
| Benchmark JSON and CSV | Local evidence under `packages/refkit-bench/results`. Keep audited code and fixtures tracked. |
| `development_docs/` | Tracked maintainer guidance. Excluded from published distributions. |

Generated output should have an authoritative input, a reproducible command, and a freshness or artifact check. Avoid hand edits when a generator owns the result.

## Documentation Ownership

- Root and package READMEs introduce installation and public package use.
- `docs/` owns public API contracts and runtime guides.
- `development_docs/` owns architecture, workflows, tests, packaging, release, and benchmark details.
- `AGENTS.md` files keep short instructions close to the code they govern.

`docs/.vitepress/config.mts` owns the rendered navigation, base path, llms plugin, and site metadata. `docs/scripts/verify-build.mjs` owns static output checks. `.github/workflows/docs.yml` owns the Pages artifact, and `ci.yml` owns deployment. The [documentation site guide](documentation.md) owns the build and browser workflow.

Root `README.md` is the single public entry point allowed to link into the developer index. Public package READMEs remain self-contained because package indexes render them outside the repository.

## GitHub Actions

Pin third-party actions to full commit SHAs. Build the shared `refkit-tests` support wheel through `test-support.yml`. Reuse each package artifact workflow for CI and publication, with build jobs feeding installed-artifact tests. The three package publish jobs run independently, then join at release completion. Native builds configure Rust path remapping before compilation. Publish jobs validate the merged archive set before trusted publication.

The shared `Benchmarks` workflow fingerprints runtime, build, dependency, and harness inputs, reuses matching evidence, and measures release builds on Linux, Windows, and macOS when needed. Main and release workflows consume the consolidated artifact. `scripts/tests/test_workflow_contract.py` checks its matrix, failure handling, rerun artifact contract, and the trusted commit publisher's checkout and permissions. These tests run through `make test` in source checks. See [benchmarks](benchmarks.md#main-branch-and-release-benchmarks) for selection, comparisons, artifacts, and comments.
