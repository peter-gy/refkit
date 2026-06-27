# Pyodide Packaging

`refkit` works in Pyodide through the same package split used on CPython:

| Package | Role |
| --- | --- |
| `refkit` | Pure Python package. It depends on one exact `refkit-core` version and checks that version at import time. |
| `refkit-core` | Native Rust/PyO3 extension package. It builds `refkit_core._refkit_core` for CPython and PyEmscripten. |
| `polars-refkit` | Native Polars expression plugin for CPython. It is not part of the Pyodide wheel lane. |

Pyodide resolves `refkit` the same way CPython does. The pure Python wheel declares `refkit-core==<same release>`, and Pyodide selects the matching `pyemscripten` wheel for its runtime.
The current lane targets Python 3.14 and the `pyemscripten_2026_0_wasm32` ABI.

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
cd packages/refkit-core
maturin build --release \
  --target wasm32-unknown-emscripten \
  --interpreter 3.14
```

The workflow does not hardcode Emscripten, Rust nightly, ABI version, or cargo rustflags.
Those values are part of the Pyodide runtime contract.

## Smoke Test

The Pyodide test lane installs the built `refkit-core` wheel, installs the local pure Python `refkit` package, and verifies the public import path:

```python
import refkit as rk

assert rk.check_refkit_core_version()
library = rk.Library.parse_bibtex("@article{doe2024, title={Fast Citations}, year={2024}}")
rendered = rk.Document(library, rk.Style.load("apa"), locale="en-US").render(
    [rk.Citation("intro", "doe2024")]
)
assert rendered["intro"].text
assert rk.build_info
```

This is a packaging smoke test. Full API behavior is still covered by the normal CPython test suite, which exercises the same Rust core and Python wrapper contracts.
