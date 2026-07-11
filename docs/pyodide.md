# Use RefKit In Pyodide

`refkit`, `refkit-core`, and `polars-refkit` use the same package and import names on CPython and Pyodide. The current PyEmscripten wheels target Pyodide 314.0.2 with Python 3.14.

## Load RefKit In A Browser Or Node

After creating a `pyodide` instance with `loadPyodide`, load `micropip` and install `refkit`:

```javascript
await pyodide.loadPackage("micropip");
await pyodide.runPythonAsync(`
import micropip
await micropip.install("refkit")
`);
```

`micropip` installs the exact matching `refkit-core` release and selects its PyEmscripten wheel. Then run the regular Python API inside Pyodide:

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

Parsing, raw BibTeX editing, formatting, citation rendering, bibliography rendering, and structured rendered output use the same APIs as CPython.

## Add The Polars Expressions

The Polars plugin ABI must match the Polars wheel loaded by Pyodide. Install the tested Python package pair for the current runtime:

```javascript
await pyodide.loadPackage("micropip");
await pyodide.runPythonAsync(`
import micropip
await micropip.install(["polars==1.33.1", "polars-refkit"])
`);
```

Then use the normal expression namespace:

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
    citation=pl.col("bibtex").refkit.cite("key", style="apa"),
).to_dicts()[0]

assert row == {"count": 1, "citation": "(Doe, 2024)"}
```

Use Pyodide 314.0.2, Polars 1.33.1, and a `polars-refkit` wheel for the `pyemscripten_2026_0` platform together. A different Polars wheel can fail plugin loading even when its Python package version satisfies the broad dependency range.

## Use The Pyodide CLI

The Pyodide CLI provides a Python command for testing, experimentation, and bundle preparation. Inside its virtual environment, install the same packages with pip:

```bash
python -m pip install refkit
python -m pip install 'polars==1.33.1' polars-refkit
```

If installation reports that no compatible wheel exists, use the supported Pyodide runtime or select a RefKit release built for that runtime.

See the official [Pyodide package loading guide](https://pyodide.org/en/stable/usage/loading-packages.html) for `micropip` and JavaScript loading patterns. See the [API contracts](api-contracts.md) for RefKit return shapes and errors.
