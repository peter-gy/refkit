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

Both package-local PEP 517 backends compose Maturin with `agent-plugins`. Each wheel carries its plugin manifest and exact package-specific skill tree. A source distribution stages the same files under `.agent-plugin` so a wheel rebuilt from that archive uses the captured release resources. An editable install stores a marker for the authored plugin root.

GitHub's native and PyEmscripten jobs call Maturin directly for cross-platform wheel production. `agent-plugins attach-wheel` adds the configured plugin to each prebuilt wheel before SBOM normalization and archive validation.

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

Package artifact workflows consume built artifacts through clean environments:

- `artifacts-refkit.yml` builds and exercises `refkit` wheels and sdists on CPython and Pyodide.
- `artifacts-polars-refkit.yml` builds the plugin and tests compatible Polars versions on CPython and Pyodide.

`test-support.yml` builds the internal `refkit-tests` wheel once per CI or tag run. Both package artifact workflows install it alongside candidate distributions. Shared probes and runtime tests live in `packages/refkit-tests/src/refkit_tests`. They run outside the checkout through `python -m refkit_tests.smoke_refkit` or `python -m refkit_tests.smoke_polars_refkit`, followed by their package-specific runtime tests.

The Pyodide lane creates a virtual environment from the pinned xbuild environment, installs locked runtime packages, the support wheel, and local candidate wheels, then executes the same probes and focused runtime tests. The runtime lock includes `agent-plugins` so the local RefKit wheel can resolve its Agent Skill without package-index access.

## Artifact provenance

Each build records a manifest with the source revision, exact build constraints, tool versions, archive filenames, and SHA-256 hashes. Publication verifies the merged package artifact set before uploading it. Package build compatibility bounds remain separate from the pinned release environment.

Native wheel jobs cover Linux, macOS, and Windows in both PR and release runs. Wheels rebuilt from sdists execute the same installed behavior probes as direct wheels.

## Publish Dependencies

`.github/workflows/publish.yml` validates the tag, then allows source checks and package artifact builds to run in parallel. It performs these stages:

1. Build and test the `refkit` sdist, CPython wheels, and PyEmscripten wheel.
2. Build and test the `polars-refkit` sdist, CPython wheels, and PyEmscripten wheel.
3. Publish each validated distribution.
4. Join both publish jobs at the release-complete check, then update release notes.

Build jobs import each sdist, test each wheel, and upload the artifacts. The same package artifact workflows run for pull requests and publication. Installed probes in `refkit_tests` exercise parsing, rendering, raw or column operations, reports, and the packaged agent examples. Publish jobs download the merged wheel and sdist sets, validate every archive, and use OpenID Connect (OIDC) trusted publishing so the workflow exchanges its GitHub identity for a short-lived package-index credential.

## Release Completion

Before a release tag, run `make check` and validate the intended tag with the release contract. After publication, verify each public package version and install path from a clean environment. Handle a partial publish as external release state and preserve the same version during recovery.

## Recover A Partial Publish

1. Record which distribution version reached the package index and which publish job failed.
2. Preserve the released version. Package indexes do not permit replacing an existing archive under the same filename.
3. Re-run the failed package's artifact validation against the exact retained workflow artifacts.
4. Publish the missing distribution with the same synchronized version when its artifacts remain valid.
5. Run the release-complete checks and verify clean installs of both distributions.

Escalate to a new version only when the retained artifact is invalid or the package index rejects the recovery upload.
