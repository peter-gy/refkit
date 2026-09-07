---
description: Diagnose parsing, raw ambiguity, rendering, Polars plugin, source-build, and Pyodide failures.
---

# Troubleshooting

Start with the boundary that produced the failure. Preserve the original bibliography source and gather the smallest diagnostic before changing it.

## A `.bib` file raises `RefkitError`

Run the same parse with report recovery:

```python
library = rk.Library.read("references.bib", recovery="report")
print(library.diagnostics)
```

If the source produces recoverable entries, fix the reported blocks and return to the default error policy. If it produces no entries, inspect the source-order failure through `BibDocument.failed_blocks`.

## A raw key is ambiguous

Use occurrence access:

```python
entries = document.entries.get_all("duplicate-key")
fields = entries[0].fields.get_all("title")
```

Choose the occurrence by source order or byte span before assigning a value.

## A citation key is missing

Inspect `library.keys()` before rendering. `Document.render` raises `MissingReferenceError` before returning partial citation output.

## A style or locale fails

Use `Style.load(name)` and `Locale.load(code)` separately to validate bundled identifiers before creating a `Document`. Use `Style.from_path(path)` for an independent CSL file.

Dependent CSL styles require parent resolution and are rejected by the explicit-style constructors.

## A Polars result is null

Check parsing first:

```python
frame.select(pl.col("bibtex").refkit.parse_report(recovery="report"))
```

A successful parse with a null render result usually indicates a missing key, null key input, or render failure. Use `render_report` with a list of keys to distinguish parsing, missing-key, and rendering failures:

```python
frame.select(pl.col("bibtex").refkit.render_report(pl.concat_list("key")))
```

## A Polars query raises `ColumnNotFoundError`

A string argument names a column. Wrap literal source and keys:

```python
pl.lit(source).refkit.cite(pl.lit("doe2024"))
```

## A Polars query raises `DuplicateError`

Alias repeated expressions with the same default output name:

```python
pl.col("bibtex").refkit.cite("key").alias("primary_citation")
```

## The Polars plugin will not load

Confirm that the installed Python package and native wheel come from one `polars-refkit` release. In Pyodide, use the documented Python, PyEmscripten, and Polars compatibility tuple.

Reinstall both the package and matching Polars version in a clean environment when the application binary interface changed.

## Import reports a RefKit version mismatch

The `refkit` Python package checks the native extension version during import. Remove mixed editable and installed copies, then reinstall one complete release:

```bash
python -m pip install --force-reinstall refkit
```

## Pip starts a source build

Pip builds from the source distribution when no compatible wheel is available. Install a Rust toolchain, Git, and a working Python build environment, or choose a platform and Python version covered by the release wheels. Source builds fetch the pinned Hayagriva and Citationberg Git revisions from GitHub unless they are cached.

## Pyodide cannot find a compatible wheel

Use the compatibility set in [Run in Pyodide](/pyodide). A wheel built for another PyEmscripten platform or Polars plugin ABI cannot load in the current runtime.
