# Rust quality policy

Use Rust 1.95 or newer for every workspace. CI checks both the shared minimum and current stable Rust. After [setting up the workspace](development.md), run these commands from the repository root on your Rust build host. Native Rust tests need the Python development library for the selected interpreter.

```bash
make rust-tools
make rust-lint rust-audit rust
```

`rust-tools` installs pinned versions of cargo-shear and cargo-deny from their locked dependencies. It requires network access. `rust-audit` verifies those versions before checking dependencies and refreshing the RustSec advisory database.

## Source policy

The root `Cargo.toml` defines the canonical Rust and Clippy lint tables. The JavaScript and Polars workspaces carry matching tables because Cargo does not inherit lint settings across workspaces. Every package inherits its workspace table. `scripts/rust_quality_contract.py` rejects drift.

Clippy runs `all` and `pedantic` at deny level, with selected restriction lints for unchecked panics, indexing, debugging residue, placeholder code, unsafe operations, wildcard imports, unnecessary cloning, and large allocations. Functions are limited to 100 lines and nesting depth four. Standard domain names such as `value` and `result` remain valid. Placeholder names such as `foo`, `thing`, and `stuff` are rejected.

Public Rust items require documentation. Fallible APIs describe their errors, and documented panic conditions must explain an invariant rather than substitute for input validation.

Tests use Clippy's explicit test configuration for fixture `unwrap`/`expect` calls, panics, and indexing. Production code retains the corresponding deny rules. Debug output and placeholder macros remain denied in tests. Assertion messages should add failure context rather than repeat an assertion's operands.

## Exceptions

First-party source cannot use `allow` attributes or crate-wide expectations. A local `expect` must identify specific lints and explain the invariant or false positive. Broad lint-group expectations and dead-code exceptions are rejected. Unfulfilled expectations fail compilation so obsolete exceptions must be removed.

Keep exceptions next to the constrained function or statement. Fix missing validation and unnecessary allocations before considering an exception. A checked, immutable source-span invariant differs from an unchecked caller-provided index.

## Dependency checks

Each workspace owns its `deny.toml`. The checks reject security advisories, yanked dependencies, unapproved licenses and sources, wildcard dependency requirements, and duplicate versions without a specific exception. Maintenance-only advisories for pinned upstream dependencies and unavoidable duplicate major versions carry individual reasons. Unused advisory and license exceptions fail the check.

Cargo-shear checks dependency usage, source reachability, and redundant ignores with `--deny-warnings`. All three workspaces are checked, including the adapter-local dependency graphs.

## Build and compatibility boundaries

Both native adapters use the `abi3` feature for the Python 3.10 stable ABI. Maturin selects extension linking through `PYO3_BUILD_EXTENSION_MODULE` while building wheels. Cargo tests run with all features and ordinary Python linking. Installed CPython and Pyodide tests verify the extension boundary separately.

For a Rust API comparison, install the pinned checker with `make rust-semver-tools`, then run `make rust-semver BASE_REV=...` with an explicitly reviewed tag or commit. This checks the portable Rust API, not Python or TypeScript compatibility. Deliberately breaking releases need a versioning decision, not a comparison against the current commit to manufacture a pass.

Keep commit hooks limited to formatting and Clippy when using a local Rust build host. Dependency audits, compatibility checks, artifacts, and runtime tests belong in CI. For remote-only development, run these commands on the build host rather than configuring a hook that compiles on the source-editing machine.
