# RefKit Architecture

RefKit keeps bibliography semantics in one portable Rust core. Python and Polars adapt the portable core API to their host runtimes.

## Vocabulary And Boundaries

Use these terms consistently across source, tests, and documentation:

| Term | Meaning |
| --- | --- |
| Capability | A user behavior such as parsing, rendering, formatting, inspecting, or editing. |
| Interface | A public call surface. Current interfaces are Python objects and Polars expressions. |
| Adapter | Code that converts host inputs and outputs to and from the portable core API. |
| Distribution | An installable package such as `refkit` or `polars-refkit`. |
| Crate | A Rust package. `refkit-core` owns portable behavior. |
| Native module | A compiled Python extension such as `refkit._native`. |
| Plugin | The compiled Polars expression library loaded by Polars. |

RefKit is the product and repository. `refkit` is both a Python distribution and import. `polars-refkit` is a Python distribution, while `polars_refkit` is its import.

A bibliography source is text or a file in BibTeX, BibLaTeX, or Hayagriva bibliography YAML form. A normalized bibliography is the `Library` representation used for lookup and rendering. A raw BibTeX document is the source-order `BibDocument` representation used for preserving edits.

## Dependency Direction

```text
                    crates/refkit-core
                       ^           ^
                       |           |
          packages/refkit     packages/polars-refkit
                 ^                     ^
                 |                     |
          Python callers          Polars query plans

packages/refkit-bench, scripts, and GitHub Actions consume the public packages
from outside the runtime graph.
```

Dependencies point inward toward `crates/refkit-core`:

- [`crates/refkit-core`](../crates/refkit-core) owns in-memory bibliography behavior and RefKit-owned port types.
- [`packages/refkit`](../packages/refkit) owns the Python adapter, filesystem access, native module registration, Python helpers, stubs, and the `refkit` distribution.
- [`packages/polars-refkit`](../packages/polars-refkit) owns the Polars adapter and its package-local Rust workspace.
- [`packages/refkit-bench`](../packages/refkit-bench) measures public workflows through concrete comparison adapters.
- [`scripts`](../scripts) and [GitHub Actions](../.github/workflows) compose validation, packaging, and release operations.

`scripts/architecture_contract.py` checks the permitted dependency sets, adapter paths, workspace membership, released engine sources, and host-boundary ownership.

## Portable Core API

The portable core API of `crates/refkit-core` accepts text or bytes and returns RefKit-owned records:

- `Library::parse_biblatex(source, RecoveryPolicy)` parses normalized BibTeX or BibLaTeX.
- `Library::parse_hayagriva_yaml(source)` parses normalized Hayagriva YAML.
- `RawDocument::parse(source)` preserves raw blocks, occurrences, spans, and edits.
- `Document` renders an ordered citation document from a library, style, and locale.
- `tidy_bibtex(source, options)` formats raw BibTeX and returns structured warnings.
- `decode_bibliography(bytes)` performs deterministic UTF-8 and Windows-1252-compatible decoding in memory.

Hayagriva, BibLaTeX, Citationberg, and serializers are pure in-process implementation libraries. Their concrete types stay behind RefKit-owned records. A trait port belongs in the core when the core must call replaceable I/O or a runtime service. Keep a pure dependency direct until a second implementation or runtime selection creates a substitution boundary.

## Boundary Records

Core records describe bibliography behavior before an adapter chooses a host shape:

- `EntryRecord` becomes Python `Entry` objects or Polars entry structs.
- `ParseReport` becomes Python diagnostics or a Polars parse-report struct.
- `RawBlockInfo` and raw occurrence records become Python dictionaries and live raw handles.
- `RenderedRecord` and `RenderedNode` become Python tree dictionaries. Polars rendered expressions project text and HTML into a struct.
- `TidyResult` and `TidyWarning` become Python objects or Polars report fields.

Keep Python dictionary keys, Polars dtype construction, JSON encoding, and host exceptions in adapters. Add a core record when more than one interface can reasonably consume the same semantic result.

## Adapters And Composition

### Python

`packages/refkit/rust` translates the portable core API into PyO3 classes, Python exceptions, dictionaries, and rendered trees. `filesystem.rs` reads paths, infers bibliography formats, attaches path-specific decode diagnostics, and writes raw BibTeX. `module.rs` registers the native classes and functions as `refkit._native`.

`packages/refkit/src/refkit/__init__.py` is the Python composition root. It exposes the supported native objects and adds the path-based `cite`, `full_bibliography`, and `tidy_file` helpers.

### Polars

`packages/polars-refkit/polars_refkit` builds expressions and registers `pl.Expr.refkit`. The Rust plugin receives Arrow-compatible values, translates the `recovery` transport value into `RecoveryPolicy`, broadcasts inputs, calls the core, and returns Polars-native scalars, lists, or structs.

The Polars workspace owns its Polars, PyO3, and `pyo3-polars` application binary interface family. It depends on the shared core through a path dependency while retaining a package-local lockfile.

## State Owners

| Owner | Contract |
| --- | --- |
| `Library` | Normalized entries and parser diagnostics. |
| `BibDocument` | Source-order raw BibTeX, occurrence identity, and edit-preserving writeback. |
| `Style` | A prepared Citation Style Language style. |
| `Locale` | A validated bundled locale code wrapper in the Python adapter. |
| `Document` | Prepared library, style, and optional locale inputs. |
| Render or bibliography call | Fresh processor state for one ordered operation. |
| Python filesystem adapter | Path reads, extension detection, decode context, and writes. |
| Python composition root | Public import surface and one-call helpers. |
| Polars adapter | Expression registration, broadcasting, dtype shape, and row failure mapping. |
| Release contract | Synchronized package and Rust crate versions. |

One transition has one owner. Adapter code converts values and lifecycle. Bibliography rules belong in the core.

## Bibliography State Models

`Library` is the normalized citation database used for selection, projection, rendering, and normalized export.

`BibDocument` is the raw BibTeX document used when comments, preambles, string definitions, malformed blocks, source order, duplicate occurrences, byte spans, or field-preserving writeback matter.

The models share syntax and normalization code where their contracts overlap. They retain distinct state because normalized entries cannot represent every raw source block.

## Main Flows

### Read And Normalize

`Library.read(path)` enters through the Python filesystem adapter. The adapter reads bytes, asks the core to decode them, infers BibLaTeX or Hayagriva YAML from the extension, and calls the matching core parse operation. The returned `Library` stores core parser diagnostics plus any path-specific decode diagnostic.

In-memory callers enter directly through `Library.parse_bibtex` or `Library.parse_yaml`, which map Python arguments to the same core operations.

### Render A Document

`Document` stores immutable library and style handles plus an optional locale code. `Document.render` creates a fresh driver, resolves the complete ordered citation list, and returns named citations plus the cited bibliography. Separate calls are independent. The core returns text, HTML, and typed rendered nodes. Adapters convert those records into host values.

### Edit Raw BibTeX

`BibDocument` scans source-order blocks and indexes entry and field occurrences. A `BibField.value` assignment validates a replacement against the original delimiter mode and records a value-span patch. Serialization applies changed spans to the original entry slices and preserves unrelated blocks.

### Execute A Polars Expression

The Python namespace translates columns or literals into a plugin call. Rust receives row values, applies broadcasting, calls the core per bibliography source, and builds the declared Polars dtype. Value expressions map row parse failures to null. Report expressions return diagnostic detail.

## Placement Rules

- Put reusable bibliography behavior and stable typed records in `crates/refkit-core`.
- Put filesystem access, Python conversion, exceptions, GIL policy, and native registration in `packages/refkit`.
- Put dataframe broadcasting, dtype construction, and row-failure behavior in `packages/polars-refkit`.
- Keep benchmark orchestration and comparison-package behavior in `packages/refkit-bench`.
- Keep build, release, and archive policy in `scripts`, package manifests, and GitHub Actions.

A public change is complete when runtime exports, stubs, documentation, boundary tests, and every affected adapter agree. Add semantic behavior to the core before exposing it through another host.
