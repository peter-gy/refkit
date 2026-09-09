---
description: Inspect the source-workspace Rust API that supplies RefKit's portable bibliography capabilities.
---

# Rust Core

The `refkit-core` crate is RefKit's portable, adapter-facing Rust API inside the source workspace. It accepts in-memory values and returns RefKit-owned records. Host paths, Python and JavaScript objects, and Polars values stay in their adapters.

Use the crate from a matching RefKit source or Git revision with Rust 1.88 or newer. The versioned bindings install through the `refkit` and `polars-refkit` Python distributions and the `refkit-js` npm package.

## Public capability groups

| Capability | Main exports |
| --- | --- |
| Normalized parsing | `Library`, `RecoveryPolicy`, `Diagnostic`, `ParseFailure`, `ParseReport`, `EntryRecord`, `EntryField`, `parse_bibtex_report` |
| Raw BibTeX | `RawDocument`, `ResolvedBibEntry`, occurrence IDs, inspection records, and edit errors |
| Rendering | `Document`, `Cite`, `CitationRequest`, `RenderedDocument`, `RenderedOutput`, render functions, and render errors |
| Styles | `PreparedStyle`, `load_prepared_style`, `prepare_style_from_xml`, and `StyleError` |
| Render tree | `RenderedRecord`, `RenderedNode`, `RenderedFormatting`, and `BibliographyLayout` |
| Formatting | `TidyOptions`, `TidyResult`, `TidyWarning`, `TidyRename`, duplicate rules, merge strategies, and `tidy_bibtex` |
| Decoding | `DecodedText`, `TextEncoding`, and `decode_bibliography` |

## Resolve source fields

`RawDocument::resolve(&self) -> Result<Vec<ResolvedBibEntry>, ParseFailure>`
expands string macros and concatenations in current source fields. Each record
contains `key`, `entry_type`, and a `BTreeMap<String, String>` of `fields`.
The method preserves TeX text, custom fields, and entry order and leaves the
document unchanged. See [field resolution](/guides/edit-bibtex#resolve-fields-for-inspection)
for macro lookup and diagnostic semantics.

## Adapter boundary

The core API owns bibliography semantics and typed records. An adapter owns:

- Filesystem reads and writes.
- Host-specific objects and error classes.
- Serialization into dictionaries, structs, or other host values.
- Runtime registration and lifecycle.

The Python, JavaScript, and Polars adapters depend on this API. A new adapter should preserve the same capability meanings while choosing host-native inputs, outputs, and failure behavior.

## State model

Core `Library` owns normalized entries and parser diagnostics. Core `RawDocument` owns raw syntax, occurrences, and edits. Core `Document` stores prepared rendering inputs. Each render or bibliography call creates fresh processor state.

The crate serves the workspace adapter boundary. Adapter authors should pin the complete RefKit revision so the record and capability versions remain aligned.

See [How RefKit Works](/concepts/how-refkit-works) for the product model and the repository's `development_docs/` for contributor architecture.
