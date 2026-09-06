---
description: Follow RefKit's normalize, render, and raw-edit flows through their state owners.
---

# How RefKit Works

RefKit moves bibliography data through three flows. Each flow has its own state owner and observable result.

<div class="model-flow">
  <div>
    <strong>Normalize</strong>
    <p>BibTeX, BibLaTeX, or Hayagriva YAML becomes a <code>Library</code> of normalized entries and parser diagnostics.</p>
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

`Library` is the normalized citation database. It accepts BibTeX or BibLaTeX source, Hayagriva YAML, or a supported file path. Each `Entry` exposes a citation key, entry type, common fields, and normalized parent records.

Normalization resolves bibliography syntax into records suited to selection, projection, and rendering. `Library.diagnostics` records messages from recoverable BibTeX parsing.

## Preserve raw BibTeX

`BibDocument` scans the original source into ordered blocks. An occurrence is one entry or field at one source position. Occurrence identity keeps duplicate entry keys and duplicate field names addressable without collapsing them.

Assigning `BibField.value` changes the shared document state. `BibDocument.to_bibtex()` renders that state in memory. `BibDocument.write(path)` writes it to a path.

## Render an Ordered Citation Document

`Style` prepares a Citation Style Language (CSL) style. `Locale` validates and stores a bundled locale code. The renderer loads that locale's terms when a `Document` operation starts.

`Document.render` accepts ordered, named `Citation` objects. A `CitationGroup` holds one or more `Cite` items. The order can affect disambiguation and the cited bibliography.

```text
Library + Style + Locale + ordered Citation values
                    ↓
                 Document
                    ↓
     named citations + cited bibliography
```

Each output is `Rendered`. It exposes the same result as plain text, escaped HTML, and a structured render tree.

## Adapt the core to a host

The `refkit-core` Rust crate owns parsing, recovery, raw syntax, formatting, styles, and rendering. An adapter translates those capabilities into host-native values:

| Host | Adapter owns |
| --- | --- |
| Python | Paths, Python objects, exceptions, native module registration, and one-call helpers. |
| Polars | Expressions, broadcasting, column dtypes, plugin loading, and row failure mapping. |
| Pyodide | The Python interfaces delivered as PyEmscripten wheels for an Emscripten-based Python runtime. |

Continue with [Choose a Bibliography Model](/concepts/bibliography-models) to choose between normalized data and raw source.
