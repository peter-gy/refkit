---
description: Install RefKit PyEmscripten wheels in the tested Pyodide and Polars runtime set.
---

# Run in Pyodide

[Pyodide](https://pyodide.org/) runs Python and native Python packages compiled to WebAssembly in a browser or Node.js. RefKit publishes PyEmscripten wheels, the wheel format used by Python extensions compiled through Emscripten for this runtime.

The current compatibility set is:

| Component | Tested value |
| --- | --- |
| Python | 3.14 |
| Pyodide xbuild environment | 314.0.2 |
| PyEmscripten platform | `pyemscripten_2026_0_wasm32` |
| Polars for `polars-refkit` | 1.33.1 |

The xbuild environment pins the compiler, runtime application binary interface, and build flags. It is a build-environment version rather than the Pyodide product version.

## Load `refkit`

Create a Pyodide instance with `loadPyodide`, then install through [micropip](https://micropip.pyodide.org/):

```javascript
await pyodide.loadPackage("micropip")
await pyodide.runPythonAsync(`
import micropip
await micropip.install("refkit")
`)
```

Run the regular Python API inside that instance:

```python
import refkit as rk

library = rk.Library.parse_bibtex(
    """
@article{doe2024,
  author = {Doe, Jane},
  title = {Browser Citations},
  year = {2024}
}
"""
)
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
rendered = document.render([rk.Citation("intro", "doe2024")])

print(rendered["intro"].text)
```

Expected output:

```text
(Doe, 2024)
```

Parsing, raw BibTeX editing, formatting, citation rendering, bibliography rendering, and structured output use the same Python API as CPython.

## Add Polars expressions

The Polars Python wheel and the `polars-refkit` native plugin must share a compatible plugin application binary interface.

```javascript
await pyodide.loadPackage("micropip")
await pyodide.runPythonAsync(`
import micropip
await micropip.install(["polars==1.33.1", "polars-refkit"])
`)
```

Then import the normal packages:

```python
import polars as pl
import polars_refkit

frame = pl.DataFrame(
    {
        "bibtex": [
            "@article{doe2024, author={Doe, Jane}, title={Browser Citations}, year={2024}}"
        ],
        "key": ["doe2024"],
    }
)

row = frame.select(
    count=pl.col("bibtex").refkit.entry_count(),
    citation=pl.col("bibtex").refkit.cite("key"),
).to_dicts()[0]

assert row == {"count": 1, "citation": "(Doe, 2024)"}
```

A different Polars wheel can satisfy the Python version range and still fail native plugin loading. Keep the documented Polars and `polars-refkit` pair together.

## Use the Pyodide CLI

The Pyodide CLI creates a virtual environment that runs the PyEmscripten runtime. Install the pinned build tool, create the environment, and activate it before installing RefKit:

```bash
python -m pip install 'pyodide-build==0.35.1'
pyodide xbuildenv install 314.0.2
pyodide venv .venv-pyodide
. .venv-pyodide/bin/activate
python -m pip install refkit
python -m pip install 'polars==1.33.1' polars-refkit
```

An installation error that reports no compatible wheel means the package index has no wheel for the active PyEmscripten platform. Use the tested compatibility set or choose a RefKit release built for that runtime.

The [Pyodide package loading guide](https://pyodide.org/en/stable/usage/loading-packages.html) covers JavaScript initialization, package repositories, and `micropip` behavior.
