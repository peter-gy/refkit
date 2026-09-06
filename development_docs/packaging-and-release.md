# Packaging And Release

RefKit publishes two Python distributions from one synchronized release version. `refkit` contains the Python API and native bibliography extension. `polars-refkit` contains the native Polars plugin.

A wheel is an installable Python archive. A source distribution, or sdist, contains source for a package build. ABI3 is Python's stable native extension application binary interface, which lets one compatible wheel cover several CPython releases.

## Artifact Graph

| Distribution | Artifacts | Runtime relationship |
| --- | --- | --- |
| `refkit` | sdist, CPython ABI3 wheels, PyEmscripten wheel | Provides `refkit._native`, the public Python API, and its version-matched Agent Plugin. |
| `polars-refkit` | sdist, CPython wheels, PyEmscripten wheel | Requires a compatible Polars runtime and provides `polars_refkit._internal`. |

Build local CPython artifacts with:

```bash
make build
```

The target clears prior distribution output, builds both packages, normalizes native wheel software bills of materials, and runs `scripts/distribution_contract.py` over every archive.

## Version Contract

`scripts/release_contract.py` keeps these sources aligned:

- root Cargo workspace
- root Python workspace
- `refkit` and its native Rust crate
- `polars-refkit` and its native Rust crate
- the shared `refkit-core` Rust dependency

Release tags use `vX.Y.Z` or `vX.Y.Z-rc.N`. Validate a prepared tag locally with:

```bash
uv run --locked --all-packages --group dev \
  python scripts/release_contract.py --tag vX.Y.Z
```

Creating or pushing a release tag changes public package state. Confirm release authorization and the intended version before that step.

## Native Build Contract

Native release builds use locked Cargo resolution and path remapping. Remapping removes checkout, Cargo registry, Git checkout, Rust toolchain, and builder-home paths from compiled artifacts.

Maturin is the Rust-backed Python package builder used by both distributions. It emits a software bill of materials (SBOM), an inventory of components inside the wheel. `scripts.normalize_wheel` replaces local references with stable package references, removes generated timestamps and serial numbers, and refreshes the affected wheel `RECORD` hashes. The distribution contract then rejects generated Python bytecode, developer documentation, local SBOM references, and embedded builder paths.

The `refkit` PEP 517 backend composes Maturin with `agent-plugins`. A regular wheel carries `plugin.json` and the exact `skills/refkit` tree beside the Python package. A source distribution stages the same files under `.agent-plugin` so a wheel rebuilt from that archive uses the captured release resources. An editable install stores a marker for the authored plugin root.

GitHub's native and PyEmscripten jobs call Maturin directly for cross-platform wheel production. `scripts.augment_agent_plugin` applies the same public Agent Plugins build adapter to those prebuilt wheels before SBOM normalization and archive validation.

Local `make build` normalizes and validates both wheel and sdist contents. The publish workflow uploads build artifacts, downloads the complete merged set, then runs `twine check --strict` and the distribution contract immediately before trusted publication.

## PyEmscripten Builds

[Pyodide](https://pyodide.org/) runs Python compiled through Emscripten in WebAssembly hosts. A PyEmscripten wheel contains a native Python extension built for that target. The xbuild environment pins the cross-build compiler, runtime application binary interface, and flags used to create those wheels.

The runtime source is `.github/pyodide/runtime.json`. The current contract targets Python 3.14 and records the xbuild environment, Polars wheel tag, and tested Polars plugin application binary interface family. `.github/actions/setup-pyodide` reads the corresponding Rust toolchain, Emscripten version, Pyodide ABI, and Rust flags from the pinned xbuild environment.

The essential build inputs are:

```bash
pyodide config get rust_toolchain
pyodide config get emscripten_version
pyodide config get pyodide_abi_version
pyodide config get rustflags
```

Maturin receives the Python version, `wasm32-unknown-emscripten` target, PyEmscripten platform version, and Cargo Rust flags from those values. The Polars plugin ABI is a cross-file contract: Python Polars is pinned in `.github/pyodide/requirements.in`, while Rust Polars, PyO3, and `pyo3-polars` are pinned in `packages/polars-refkit/rust`. `runtime.json` records the tested mapping and `make pyodide-lock-check` validates both sides.

`make pyodide-lock` regenerates `.github/pyodide/pylock.314.toml` from `requirements.in` and the configured runtime. The check mode validates exact requirements, wheel hashes, source hosts, and the expected Polars wheel tag.

## Installed-Artifact Tests

Release-test workflows consume built artifacts through clean environments:

- `release-tests-refkit.yml` installs and exercises `refkit` wheels on CPython and Pyodide.
- `release-tests-polars-refkit.yml` installs compatible Polars versions and exercises the plugin on CPython and Pyodide.

The Pyodide lane creates a virtual environment from the pinned xbuild environment, installs locked runtime packages and local wheels, runs smoke programs, and executes the focused runtime tests under `.github/pyodide`. The runtime lock includes `agent-plugins` so the local RefKit wheel can resolve its Agent Skill without package-index access.

## Publish Dependencies

`.github/workflows/publish.yml` validates the tag, then allows source checks and package artifact builds to run in parallel. It performs these stages:

1. Build and test the `refkit` sdist, CPython wheels, and PyEmscripten wheel.
2. Build and test the `polars-refkit` sdist, CPython wheels, and PyEmscripten wheel.
3. Publish each validated distribution.
4. Join both publish jobs at the release-complete check, then update release notes.

Build jobs import each sdist, test each wheel, and upload the artifacts. Reusable release-test workflows install and validate the exact wheels. Publish jobs download the merged wheel and sdist sets, validate every archive, and use OpenID Connect (OIDC) trusted publishing so the workflow exchanges its GitHub identity for a short-lived package-index credential.

## Release Completion

Before a release tag, run `make check` and validate the intended tag with the release contract. After publication, verify each public package version and install path from a clean environment. Handle a partial publish as external release state and preserve the same version during recovery.

## Recover A Partial Publish

1. Record which distribution version reached the package index and which publish job failed.
2. Preserve the released version. Package indexes do not permit replacing an existing archive under the same filename.
3. Re-run the failed package's artifact validation against the exact retained workflow artifacts.
4. Publish the missing distribution with the same synchronized version when its artifacts remain valid.
5. Run the release-complete checks and verify clean installs of both distributions.

Escalate to a new version only when the retained artifact is invalid or the package index rejects the recovery upload.
