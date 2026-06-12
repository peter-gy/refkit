# Release Validation

Refkit workspace releases use local gates for metadata, artifact contents, supported Python versions, registry dependency provenance, and dependency advisories.

Run the package gate:

```bash
make package-check
```

This builds the `refkit` and `polars-refkit` sdists and wheels, then verifies that:

- each sdist excludes benchmark tooling and local-only artifacts
- each wheel excludes benchmark tooling and local-only artifacts
- each sdist includes the shared `crates/refkit-core` Rust sources needed for a source build
- the Apache license file, stubs, `py.typed`, Rust sources, and package metadata are present for each package
- wheel metadata reports version `0.0.1` for each package
- wheel metadata reports the Apache-2.0 license for each package
- wheel metadata supports Python `>=3.11, <3.15` for each package
- `polars-refkit` wheel metadata requires the validated Polars `>=1.41, <1.42` line

The target builds into a clean temporary output directory and passes exact artifact paths to the inspector, so stale files in `dist/` cannot satisfy the check.

Run the Python matrix smoke:

```bash
make release-smoke
```

This installs each built wheel into fresh Python 3.11, 3.12, 3.13, and 3.14 environments through `uv`.

The `refkit` smoke imports `refkit`, imports `refkit._native`, and checks that `refkit.__version__`, `refkit._native.__version__`, and installed package metadata all equal `0.0.1`.

The `polars-refkit` smoke imports `polars`, imports `polars_refkit`, imports `polars_refkit._internal`, checks the same version contract, and runs a tiny Polars expression through the installed plugin.

The smoke command clears Python source-path environment variables, runs from a temporary directory, and checks that Python package files and native extension files point inside the fresh smoke environment.

Run dependency provenance:

```bash
make dependency-provenance
```

This checks that the locked registry crates resolve through:

- `hayagriva 0.10.0`
- `biblatex 0.12.0`
- `citationberg 0.7.0`

It also reports the YAML parser path from `hayagriva` through `serde_yaml` to `unsafe-libyaml`.

The gate asserts the exact locked crate versions before printing the `cargo tree --locked` paths.

Run the Rust floor check:

```bash
make rust-floor
```

This target runs `rustup run 1.85 cargo check --locked --workspace` against the declared Rust floor in `Cargo.toml`. If rustup does not have Rust 1.85 installed, the target installs the minimal 1.85 toolchain before running the check.

Run advisory checks:

```bash
make advisory
```

This target may download advisory databases and audit tools. It runs `cargo audit` for `Cargo.lock` and `pip-audit` for the locked Python workspace dependencies.

The Rust advisory gate currently ignores `RUSTSEC-2024-0436` for `paste 1.0.15`. `paste` is pulled transitively by the published `hayagriva 0.10.0` and `biblatex 0.12.0` crates. The advisory is an unmaintained warning, not a reported vulnerability. New Rust advisory warnings still fail the target.

Before applying that ignore, the advisory target verifies that `paste` is only present through the accepted `hayagriva` and `biblatex` dependency path. A direct `paste` dependency or a new unrelated dependent fails the provenance guard.
