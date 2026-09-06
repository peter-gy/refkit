# Capability Map

A capability is one user behavior owned by the portable core and projected through one or more interfaces. This map connects each capability to its owner, host surfaces, and strongest test boundary.

| Capability | Core owner | Python object interface | Polars expression interface | Primary evidence |
| --- | --- | --- | --- | --- |
| Decode bibliography bytes | `source` | Path readers attach decode diagnostics | Host supplies strings | Core decode tests and Python path tests |
| Parse normalized BibTeX or BibLaTeX | `library` | `Library.read`, `Library.parse_bibtex` | Parse, inspect, and render expressions | Core recovery tests plus Python and Polars boundary tests |
| Parse Hayagriva YAML | `library` | `Library.read`, `Library.parse_yaml` | Not exposed | Core and Python public tests |
| Inspect normalized entries | `library` | `Entry`, lookup, selection, projection | `keys`, `entries`, `entry_count` | Shared contract fixture plus adapter shape tests |
| Preserve raw BibTeX | `raw` | `BibDocument`, raw entries, raw fields, blocks, spans | Not exposed | Core raw tests and Python occurrence tests |
| Edit an existing raw field value | `raw` | `BibField.value`, `BibDocument.write` | Not exposed | Core delimiter tests and Python writeback tests |
| Format BibTeX | `tidy` | `tidy_bibtex`, `tidy_file`, `BibDocument.tidy` | `tidy_bibtex`, `tidy_bibtex_report` | Vendored tidy specs, corpus tests, and adapter option tests |
| Prepare a style | `style` | Bundled, path, and XML `Style` constructors | Bundled style name argument | Core style tests and adapter render tests |
| Validate a locale code | `render` archive | `Locale.load` | Not exposed as validation | Python locale tests |
| Render ordered citations | `document` and `render` | `Document.render` and `cite` | `cite`, `cite_each`, `cite_group` families | Core render tests and adapter behavior tests |
| Render cited bibliography | `document` | `RenderedDocument.bibliography`, `cited_bibliography` | Not stateful across rows | Ordered Python render tests |
| Render full bibliography | `document` and `render` | `full_bibliography` methods | `full_bibliography_*` | Shared expected output across core, Python, and Polars |
| Inspect rendered output | `render_tree` and HTML renderer | Text, HTML, and tree | Text, HTML, or `{text, html}` struct | HTML safety and exact-shape tests |
| Report row-local failures | Core result and error records | Exceptions and diagnostics | Null value results and report structs | Polars null, dtype, and query-failure tests |

## Shared And Host-Specific Contracts

Reuse bibliography inputs and expected semantic results when interfaces expose the same capability. Keep host behavior at the adapter boundary:

- Python owns exceptions, object identity, path handling, and Global Interpreter Lock release.
- Polars owns column coercion, broadcasting, lazy execution, dtypes, plugin loading, and null propagation.
- Pyodide tests own the PyEmscripten wheel and WebAssembly runtime boundary.

Add a row to this map when a new capability or interface changes the coverage graph. Update the public docs and nearest boundary tests in the same change.
