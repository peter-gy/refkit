# RefKit Architecture

RefKit gives Python programs and Polars query plans one bibliography capability set backed by a shared Rust core. The core owns parsing, recovery, normalized records, raw BibTeX editing, formatting, CSL style preparation, rendering, and rendered trees. Host adapters convert those typed results into Python objects or Polars values.

The durable advantage is semantic alignment across interfaces and runtimes. A parsing fix, rendering rule, or raw edit invariant can serve Python objects, dataframe expressions, CPython wheels, and PyEmscripten wheels through one implementation.

## Dependency Direction

```text
crates/refkit-core
├── packages/refkit-core
│   └── packages/refkit
├── packages/polars-refkit/rust
└── crates/bibtex-tidy-rs
```

- [`crates/refkit-core`](../crates/refkit-core) is portable Rust. It owns reusable bibliography behavior and typed capability records.
- [`packages/refkit-core`](../packages/refkit-core) is the native Python distribution. Its `rust/` directory implements the PyO3 adapter and its extension module is `refkit_core._refkit_core`.
- [`packages/refkit`](../packages/refkit) is the pure Python facade. It depends on the exact matching `refkit-core` version.
- [`packages/polars-refkit`](../packages/polars-refkit) is the Polars expression adapter. Its Rust code is a package-local Cargo workspace because Polars, PyO3, and `pyo3-polars` form one plugin ABI family.
- [`crates/bibtex-tidy-rs`](../crates/bibtex-tidy-rs) preserves the Rust formatter compatibility surface over the shared core.
- [`packages/refkit-bench`](../packages/refkit-bench) measures public workflows and owns comparison adapters. It is repository tooling rather than a runtime dependency.

Adapters depend on the core. The core stays independent from Python objects, PyO3, Polars dtypes, plugin registration, JSON transport shapes, and wheel metadata. `scripts/architecture_contract.py` checks the dependency and workspace parts of this graph.

## Capability Ownership

| Boundary | Owns |
| --- | --- |
| Portable core | Bibliography parsing, recovery, normalized entries, raw syntax records, field edits, tidy options, styles, locales, document state, rendering, and rendered nodes. |
| Native Python adapter | PyO3 classes, Python exceptions, Python value conversion, GIL boundaries, and native module registration. |
| Python facade | Exact native-version check, one-call path helpers, re-exports, and the public Python stub. |
| Polars adapter | Expression registration, broadcasting, dtypes, null mapping, eager and lazy execution, and row diagnostics. |
| Packaging and CI | CPython wheels, PyEmscripten wheels, sdists, archive validation, and installed-artifact tests. |

Typed capability records belong in the portable core when another interface could consume them. Adapter-specific presentation stays at the interface boundary. Examples include Python dictionaries, PyO3 classes, Polars structs, JSON strings, exception classes, and repr behavior.

## Two Bibliography Models

`Library` is the normalized citation database. Use it for parsing recoverable bibliography input, mapping access, selectors, projection, styles, citations, and bibliographies.

`BibDocument` is the raw BibTeX document. Use it when comments, preambles, string definitions, malformed blocks, source order, duplicate occurrences, byte spans, or field-preserving writeback matter.

The models share parsing and normalization machinery where the contract overlaps. They retain distinct data because normalized citation entries cannot represent every raw source block.

## Public Domain Nouns

| Noun | Contract |
| --- | --- |
| `Library` | Normalized entries used for inspection and rendering. |
| `Entry` | One normalized bibliography entry. |
| `Style` and `Locale` | Prepared CSL rendering inputs. |
| `Cite` | One citation item with a key and optional locator. |
| `CitationGroup` | An ordered group rendered as one citation. |
| `Citation` | A stable result identifier paired with a cite value. |
| `Document` | A library, style, and locale that renders a complete ordered citation list per call. |
| `Rendered` | Text, HTML, and typed tree output. |
| `RenderedDocument` | Named citation outputs plus the cited bibliography. |
| `BibDocument`, `BibEntry`, and `BibField` | Raw source views with occurrence-aware lookup and edit behavior. |
| `TidyOptions` and `TidyResult` | BibTeX formatting input and output. |

Use these nouns across Rust records, Python APIs, Polars expressions, tests, and documentation. A capability is a user behavior such as parsing, rendering, inspecting, or formatting. An interface exposes capabilities through a host environment.

## Main Data Flows

### Normalized input

`Library.read` or `Library.parse_bibtex` reads source text, applies BibTeX recovery when requested, parses through the shared Rust dependencies, normalizes entries, and stores diagnostics beside the library. The Python adapter holds the core library and materializes Python values at public access points.

### Document rendering

`Document` combines a `Library`, `Style`, and locale. `Document.render` creates a fresh driver, resolves the complete citation list in order, and produces the named citations plus cited bibliography for that call. Separate render calls are independent. The core returns text, HTML, and typed rendered nodes. The adapter exposes those values through `Rendered` and `RenderedDocument`.

### Raw BibTeX editing

`BibDocument` scans source-order blocks and indexes entry and field occurrences. A `BibField.value` assignment validates the replacement through the core and marks the affected source span. Rendering the document rewrites changed fields while retaining unrelated blocks.

### Polars expressions

The Python namespace turns columns or literals into plugin expressions. Rust receives Arrow-compatible values, broadcasts row inputs, calls the shared core per row, and returns Polars-native scalars, lists, or structs. Row parse and formatting failures map to null value results while report expressions retain diagnostic detail.

## Placement Rules

- Put reusable bibliography behavior and stable typed records in `crates/refkit-core`.
- Put Python conversion, exception mapping, and GIL policy in `packages/refkit-core`.
- Put convenience helpers and public Python re-exports in `packages/refkit`.
- Put dataframe broadcasting, dtype, and row-failure behavior in `packages/polars-refkit`.
- Keep benchmark orchestration and comparison-package behavior in `packages/refkit-bench`.
- Keep build, release, and archive policy in `scripts/`, package manifests, and GitHub Actions.

An API change is complete when runtime exports, stubs, public docs, boundary tests, and every affected interface agree. A new host interface should call the shared core capability instead of implementing a separate parser or renderer.
