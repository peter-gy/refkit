# Portable Core

`refkit-core` is the portable core API used by every host adapter.

- Expose RefKit-owned structs, enums, errors, and result records.
- Accept source text or bytes. Keep filesystem paths, Python objects, Polars values, plugin registration, and serialized host shapes in adapters.
- Keep Hayagriva, BibLaTeX, and serializer types private when a stable RefKit type can express the result.
- Use pure implementation dependencies directly. Introduce a trait port when the core must reach replaceable I/O or a runtime service.
- Preserve source spans, UTF-8 boundaries, unrelated valid entries, and deterministic diagnostic order during recovery.
- Keep raw BibTeX parsing and tidy formatting on one syntax model.
- `RawDocument.resolve()` projects current source fields through the shared bounded macro expander. Preserve TeX text and keep citation normalization in `Library`.
- Add semantic behavior here before exposing it through an adapter.

Validate bibliography macro and inheritance dependencies before upstream normalization. Keep recovery diagnostics structured and map every span to the original UTF-8 source. Recover a failing value locally while preserving valid macro definitions and unrelated entries.

Rendering uses one citation-request processor for scalar, ordered-list, grouped, and document operations. Keep one fresh driver per operation and preserve request-local note context. Map rendered item indices to the original request items and reference keys. Preserve bibliography layout and keep each label separate from entry content. Formatting and display states use RefKit-owned enums. Bound custom XML size, nodes, nesting, and per-element attributes before deserialization. Validate macro dependencies and combined rendering-element expansion limits before preparing a style.

Tidy computes duplicate groups before allocating final keys. Preserve source occurrence identity in rename reports, rewrite `crossref` and `xdata` with final keys, and sort emitted keys. Reference values preserve key case and punctuation through text formatting options. `RawEntryId.index()` addresses source-order syntax entries, so retain that vector order through planning.

Rendering changes should cover ordered citations within one render call, bibliography sorting, text, HTML, and rendered-tree output when those boundaries are affected.

Run `make rust-lint rust rust-floor` from the repository root.
