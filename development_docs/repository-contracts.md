# Repository Contracts

RefKit keeps cross-file invariants executable. Each contract has one source-level entry point, focused tests, a Make target, and a place in the complete gate.

## Contract Scripts

| Contract | Command | Protects |
| --- | --- | --- |
| Architecture | `make architecture-check` | Portable-core independence, Cargo workspace ownership, shared-core adapter dependencies, and locked native builds. |
| Documentation | `make docs-check` | Markdown-only developer docs, local link targets, and the public-to-developer audience boundary. |
| Release metadata | `make release-check` | Lockstep versions, exact native dependency pins, repository metadata, and release tag grammar. |
| Pyodide runtime | `make pyodide-lock-check` | Runtime requirements, resolved wheels, hashes, and the tested Python-to-Rust Polars plugin ABI mapping. |
| Distribution archive | `scripts/distribution_contract.py <archives>` | Bytecode exclusion, developer-doc exclusion, normalized SBOM references, and builder-path removal. |
| Wheel normalization | `python -m scripts.normalize_wheel <wheels>` | Stable SBOM references and matching `RECORD` hashes before archive validation. |

Contract diagnostics should name the offending source or archive member and return a nonzero exit status. Keep validation deterministic and free from network access. Test a new failure mode beside the script before adding it to `make check` or CI.

## Files That Change Together

### Python API

- native PyO3 definitions under `packages/refkit-core/rust/src`
- native exports in `packages/refkit-core/src/refkit_core/__init__.py`
- native stubs in `packages/refkit-core/src/refkit_core/*.pyi`
- facade exports and helpers in `packages/refkit/src/refkit`
- public tests and end-user API docs

### Polars API

- top-level functions in `packages/polars-refkit/polars_refkit/_expressions.py`
- namespace methods in `_namespace.py`
- public exports and `__init__.pyi`
- Rust expression registration and keyword records
- eager, lazy, dtype, broadcast, null, and installed-wheel tests

### Release version

- root Cargo workspace version and shared Rust dependency
- root Python workspace version
- `refkit`, `refkit-core`, and `polars-refkit` project versions
- compatibility and Polars Rust crate versions
- exact `refkit-core` dependency in `refkit`

`scripts/release_contract.py` lists the authoritative repeated sources.

### Rust dependencies

- root `Cargo.lock`
- `crates/bibtex-tidy-rs/Cargo.lock`
- `packages/polars-refkit/rust/Cargo.lock`

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
| Cargo and uv locks | Tracked resolution contracts. Validate them before tests. |
| Pyodide lock | Tracked generated runtime input. Regenerate through `scripts/pyodide_lock.py`. |
| Wheels and sdists | Derived build output. Validate archives and leave them untracked. |
| Wheel SBOMs | Derived by native builds, normalized before archive validation. |
| Benchmark JSON and CSV | Local evidence under `packages/refkit-bench/results`. Keep audited code and fixtures tracked. |
| `development_docs/` | Tracked maintainer guidance. Excluded from published distributions. |

Generated output should have an authoritative input, a reproducible command, and a freshness or artifact check. Avoid hand edits when a generator owns the result.

## Documentation Ownership

- Root and package READMEs introduce installation and public package use.
- `docs/` owns public API contracts, runtime guides, and migration paths.
- `development_docs/` owns architecture, workflows, tests, packaging, release, and benchmark details.
- `AGENTS.md` files keep short instructions close to the code they govern.

Root `README.md` is the single public entry point allowed to link into the developer index. Public package READMEs remain self-contained because package indexes render them outside the repository.

## GitHub Actions

Pin third-party actions to full commit SHAs. Keep source checks reusable, package builds separate from installed-artifact tests, and publish jobs ordered by runtime dependency. Native builds configure Rust path remapping before compilation and validate every archive before upload.
