---
description: Follow RefKit's normalize, render, and raw-edit flows through their state owners.
---

# How RefKit Works

RefKit moves bibliography data through three flows. Python and JavaScript expose the same state owners and results.

<div class="model-flow">
  <div>
    <strong>Normalize</strong>
    <p>Bibliography source becomes a <code>Library</code> of normalized entries and parser diagnostics.</p>
  </div>
  <div>
    <strong>Render</strong>
    <p>A <code>Library</code>, style, locale, and ordered citations become named citations plus a bibliography.</p>
  </div>
  <div>
    <strong>Edit</strong>
    <p>Raw BibTeX becomes a <code>BibDocument</code> whose field edits write back into source-order blocks.</p>
  </div>
</div>

## Normalize bibliography input

`Library` is the normalized citation database. It accepts [BibTeX](https://www.bibtex.org/) or [BibLaTeX](https://ctan.org/pkg/biblatex), text formats for bibliography entries, and [Hayagriva YAML](https://github.com/typst/hayagriva/blob/main/docs/file-format.md), a structured bibliography format. Each entry exposes a citation key, entry type, common fields, and normalized parent records.

Normalization resolves bibliography syntax into records suited to selection, projection, and rendering. `Library.diagnostics` records messages from recoverable BibTeX parsing. Python and Node.js also provide file-reading APIs. Browsers supply source text to the same in-memory parsing operations.

## Preserve raw BibTeX

`BibDocument` scans the original source into ordered blocks. An occurrence is one entry or field at one source position. Occurrence identity keeps duplicate entry keys and duplicate field names individually addressable.

Assigning `BibField.value` changes the document state shared by its entry and field handles. Serializing the document produces updated BibTeX text with unrelated raw blocks retained. Write that text through the host's filesystem or browser download API when the workflow needs a file.

## Render an ordered citation document

`Style` prepares a [Citation Style Language (CSL)](https://citationstyles.org/) style, a format that describes citation and bibliography rules. `Locale` validates and stores a bundled locale code. The renderer loads that locale's terms when a `Document` operation starts.

`Document.render` accepts ordered, named `Citation` objects. A `CitationGroup` holds one or more `Cite` items. The order can affect disambiguation and the cited bibliography.

```text
Library + Style + Locale + ordered Citation values
                    ↓
                 Document
                    ↓
     named citations + cited bibliography
```

Each output is `Rendered`. It exposes the same result as plain text, escaped HTML, and a structured render tree. Each render operation owns fresh citation state. Python and JavaScript reclaim object resources automatically when the objects become unreachable.

## Adapt the core to a host

The `refkit-core` [Rust](https://www.rust-lang.org/) library owns parsing, recovery, raw syntax, formatting, styles, and rendering. Adapters translate those capabilities into host-native values:

| Host | Adapter owns |
| --- | --- |
| Python | Paths, Python objects, exceptions, native module registration, and one-call helpers. |
| JavaScript | Typed objects, [WebAssembly](https://webassembly.org/) initialization for the compiled core, and Node.js file helpers. |
| [Polars](https://docs.pola.rs/), a DataFrame query engine | Expressions, broadcasting, column data types, plugin loading, and row failure mapping. |
| [Pyodide](https://pyodide.org/), Python running in a browser | The Python interfaces packaged as wheels for its WebAssembly runtime. |

Continue with [Choose a Bibliography Model](/concepts/bibliography-models) to choose between normalized data and raw source.
