# Portable Core

`refkit-core` is the portable core API used by every host adapter.

- Expose RefKit-owned structs, enums, errors, and result records.
- Accept in-memory records, source text, or bytes. Keep filesystem paths, Python objects, Polars values, plugin registration, and serialized host shapes in adapters.
- Keep Hayagriva, BibLaTeX, and serializer types private when a stable RefKit type can express the result.
- Use pure implementation dependencies directly. Introduce a trait port when the core must reach replaceable I/O or a runtime service.
- Preserve source spans, UTF-8 boundaries, unrelated valid entries, and deterministic diagnostic order during recovery.
- Keep raw BibTeX parsing and tidy formatting on one syntax model.
- `RawDocument.resolve()` projects current source fields through the shared bounded macro expander. Preserve TeX text and keep citation normalization in `Library`.
- Add semantic behavior here before exposing it through an adapter.

Raw snapshots are immutable. `apply_patch` is the structural editing boundary and returns a new document. Validate overlapping operations before applying byte edits, preserve unrelated bytes, and map every old occurrence to its new occurrence or removal. Explicit reference edits supply final values. Share reference-key encoding with tidy and reject ambiguous automatic rewrites.

Duplicate review and tidy share rule signatures. Report evidence separately from conflicting source expressions and identifier values. Merge plans require explicit conflict choices and compile to validated patches. Copy complete value expressions so macros and TeX structure retain their meaning. Keep planning inspect-only.

`Library` owns complete immutable records and derives its private engine view from them. Preserve source data before engine narrowing. Complete record text, dates, names, and containers stay structured. Scalar projections are derived views. Record snapshots have an explicit schema version and never expose engine serialization types.

Codecs use the same records and a finite format set. Keep conversion issues separate from parser recovery diagnostics. Verify encoded output by decoding it and comparing record data. Strict loss policy must fail before returning a result. Source annotations never override changed structured fields.

Validation is inspect-only. Keep format-neutral record checks separate from the pinned BibLaTeX source profile. Use owned issue codes, targets and severity. Source validation reports current-snapshot UTF-8 spans. Identifier matches are review evidence and never authorize merging records.

Validate bibliography macro and inheritance dependencies before upstream normalization. Keep recovery diagnostics structured and map every span to the original UTF-8 source. Recover a failing value locally while preserving valid macro definitions and unrelated entries.

Rendering uses one citation-request processor for scalar, ordered-list, grouped, and document operations. Keep one fresh driver per operation and preserve request-local note context. Map rendered item indices to the original request items and reference keys. Preserve bibliography layout and keep each label separate from entry content. Formatting and display states use RefKit-owned enums. Bound custom XML size, nodes, nesting, and per-element attributes before deserialization. Validate macro dependencies and combined rendering-element expansion limits before preparing a style.

Tidy computes duplicate groups before allocating final keys. Preserve source occurrence identity in rename reports, rewrite `crossref`, `xdata`, and `xref` with final keys, and sort emitted keys. Only `crossref` and `xdata` add inheritance edges. Reference values preserve key case and punctuation through text formatting options. `RawEntryId.index()` addresses source-order syntax entries, so retain that vector order through planning.

Rendering changes should cover ordered citations within one render call, bibliography sorting, text, HTML, and rendered-tree output when those boundaries are affected.

Citation purposes are RefKit-owned values mapped before engine processing. Keep purpose behavior consistent across text, HTML, and trees. Catalog metadata is owned data sorted by canonical load name.

Prepare dependent styles from supplied XML resources. Validate child and parent XML bounds, parent identity and macros before rendering. Keep requested style metadata separate from the effective parent's rules and preserve document/child/parent locale precedence.

Run `make rust-lint rust rust-floor` from the repository root.
