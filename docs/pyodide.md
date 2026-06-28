# Pyodide Packaging

`refkit`, `refkit-core`, and `polars-refkit` install in Pyodide with the same public package names used on CPython.

```bash
python -m pip install refkit
python -m pip install polars
python -m pip install polars-refkit
```

| Package | Role |
| --- | --- |
| `refkit` | Pure Python package. It depends on one exact `refkit-core` version and checks that version at import time. |
| `refkit-core` | Native Rust/PyO3 extension package. It builds `refkit_core._refkit_core` for CPython and PyEmscripten. |
| `polars-refkit` | Native Polars expression plugin. It builds `polars_refkit._internal` for CPython and PyEmscripten. |

Pyodide resolves `refkit` the same way CPython does. The pure Python wheel declares `refkit-core==<same release>`, and Pyodide selects the matching `pyemscripten` wheel for its runtime.

`polars-refkit` uses the Polars package supplied by Pyodide. Install `polars` before installing a local `polars-refkit` wheel in smoke tests or release validation. The published package keeps the same public dependency declared in its package metadata.

## Build Contract

The PyEmscripten build asks Pyodide for the current toolchain values:

```bash
pyodide config get rust_toolchain
pyodide config get emscripten_version
pyodide config get pyodide_abi_version
pyodide config get rustflags
```

CI passes those values to `emscripten-core/setup-emsdk` and `maturin`:

```bash
CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS="$(pyodide config get rustflags)"
MATURIN_PYEMSCRIPTEN_PLATFORM_VERSION="$(pyodide config get pyodide_abi_version)"
PYODIDE_PYTHON_VERSION="${PYODIDE_PYTHON_VERSION:?set the Pyodide Python version}"
cd packages/polars-refkit
maturin build --release \
  --target wasm32-unknown-emscripten \
  --interpreter "$PYODIDE_PYTHON_VERSION"
```

The release workflows set `PYODIDE_PYTHON_VERSION` from the matrix and read Emscripten, Rust nightly, ABI, and cargo rustflags from `pyodide config get`.

`packages/polars-refkit/rust` is a package-local Rust workspace. It uses the Polars plugin ABI that matches Pyodide's current Polars wheel while depending on the shared `crates/refkit-core` library for citation parsing and rendering.

## Smoke Test

The Pyodide test lane installs the built `refkit-core` wheel, the local pure Python `refkit` package, Pyodide's `polars` package, and the built `polars-refkit` wheel. It verifies the public import paths and runs Polars expression callbacks:

```python
import polars as pl
import polars_refkit
import refkit as rk

assert rk.check_refkit_core_version()
assert polars_refkit.__version__

df = pl.DataFrame(
    {
        "bibtex": ["@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}"],
        "key": ["doe2024"],
    }
)
row = df.select(
    count=pl.col("bibtex").refkit.entry_count(),
    citation=pl.col("bibtex").refkit.cite("key", style="apa"),
).to_dicts()[0]

assert row["count"] == 1
assert row["citation"] == "(Doe, 2024)"
```

This is a packaging smoke test. Full API behavior is covered by the normal CPython test suite, which exercises the same Rust core, Python wrapper, and Polars expression contracts.
