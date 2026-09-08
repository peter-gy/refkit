---
description: Diagnose parsing, raw ambiguity, rendering, browser initialization, Polars plugin, source-build, and Pyodide failures.
---

# Troubleshooting

Start with the boundary that produced the failure. Preserve the original bibliography source and gather the smallest diagnostic before changing it.

## Bibliography parsing fails

A `RefkitError` during file loading can mean the file could not be read or its extension is unsupported. Confirm the path and use `.bib`, `.yaml`, or `.yml` for normalized library loading. A `ParseError` carries diagnostics about the source itself.

Run a failing BibTeX parse with report recovery:

::: code-group

```python [Python]
import refkit as rk

source = "@book{doe2024, title={Example}, year={2024}}\n@book{broken"
library = rk.Library.parse_bibtex(source, recovery="report")
print(library.keys())  # ['doe2024']
print(library.diagnostics[0]["action"])  # dropped_block
```

```ts [TypeScript]
import * as rk from "refkit-js";

const source = "@book{doe2024, title={Example}, year={2024}}\n@book{broken";
const library = rk.Library.parseBibtex(source, { recovery: "report" });
console.log(library.keys()); // ["doe2024"]
console.log(library.diagnostics[0]?.action); // dropped_block
```

:::

If the source produces recoverable entries, fix the reported blocks and return to the default error policy. If it produces no entries, report recovery also raises `ParseError`. Inspect that exception's `diagnostics` or parse the source with `BibDocument.parse` to inspect its failed blocks through Python `failed_blocks` or TypeScript `failedBlocks`.

## A raw key is ambiguous

Use occurrence access to inspect duplicate entries and fields:

::: code-group

```python [Python]
raw = rk.BibDocument.parse("@book{duplicate, title={First}}\n@book{duplicate, title={Second}}")
entries = raw.entries.get_all("duplicate")
fields = entries[0].fields.get_all("title")
print(fields[0].value)  # First
```

```ts [TypeScript]
const raw = rk.BibDocument.parse(
  "@book{duplicate, title={First}}\n@book{duplicate, title={Second}}",
);
const entries = raw.entries.getAll("duplicate");
const fields = entries[0]!.fields.getAll("title");
console.log(fields[0]!.value); // First
```

:::

Choose the occurrence by source order or byte span before assigning a value. [Edit Raw BibTeX](/guides/edit-bibtex) explains occurrence identity and writeback.

## A citation key is missing

Inspect `library.keys()` before rendering. `Document.render` raises `MissingReferenceError` before returning partial citation output. Citation keys refer to the parsed library, so use final keys from the rename report after formatting has generated new keys.

## A style or locale fails

Use `Style.load(name)` and `Locale.load(code)` separately to validate bundled identifiers before creating a `Document`. For a custom [CSL](https://citationstyles.org/) (Citation Style Language) file, use Python `Style.from_path(path)` or Node `readStyle(path)` from `refkit-js/node`. In a browser, load XML text and pass it to `Style.fromXml`.

Dependent CSL styles require parent resolution and are rejected by the explicit-style constructors. Supply the independent parent style. [Errors and Diagnostics](/reference/errors#styles-and-rendering) lists style validation failures.

## Browser calls fail before initialization

Browser imports require `await init()` before parsing, formatting, or rendering. Follow [Initialize the module](/guides/browser#initialize-the-module) for the WebAssembly asset URL, server response, and content security policy requirements. Each worker initializes its own module.

## A Polars result is null

[Polars](https://docs.pola.rs/) is the Python dataframe integration. Use its report expressions to inspect row failures:

```python
import polars as pl
import polars_refkit  # noqa: F401

frame = pl.DataFrame({"bibtex": ["@book{doe2024, title={Example}}"], "key": ["missing"]})
print(frame.select(pl.col("bibtex").refkit.parse_report(recovery="report")))
print(frame.select(pl.col("bibtex").refkit.render_report(pl.concat_list("key"))))
```

The parse report succeeds. The render report contains `error_code="missing_key"`. A successful parse with a null value-expression render result can also indicate null key input or a render failure. Use `render_report` to distinguish these outcomes.

## A Polars query raises `ColumnNotFoundError`

A string argument names a column. Wrap literal source and keys with `pl.lit`:

```python
print(frame.select(pl.lit("@book{doe2024, title={Example}}").refkit.cite(pl.lit("doe2024"))))
```

## A Polars query raises `DuplicateError`

Alias repeated expressions with the same default output name:

```python
print(
    frame.select(
        pl.col("bibtex").refkit.cite(pl.lit("doe2024")).alias("primary_citation"),
        pl.col("bibtex").refkit.cite(pl.lit("doe2024")).alias("secondary_citation"),
    )
)
```

## The Polars plugin will not load

Confirm that the installed Python package and native wheel come from one `polars-refkit` release. In [Pyodide](/pyodide), which runs Python in the browser through WebAssembly, use the documented Python, PyEmscripten, and Polars compatibility tuple. PyEmscripten identifies the Python build platform used by those wheels.

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
