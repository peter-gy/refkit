# Portable Core

`refkit-core` is the portable core API used by every host adapter.

- Expose RefKit-owned structs, enums, errors, and result records.
- Accept source text or bytes. Keep filesystem paths, Python objects, Polars values, plugin registration, and serialized host shapes in adapters.
- Keep Hayagriva, BibLaTeX, and serializer types private when a stable RefKit type can express the result.
- Use pure implementation dependencies directly. Introduce a trait port when the core must reach replaceable I/O or a runtime service.
- Preserve source spans, UTF-8 boundaries, unrelated valid entries, and deterministic diagnostic order during recovery.
- Keep raw BibTeX parsing and tidy formatting on one syntax model.
- Add semantic behavior here before exposing it through an adapter.

Rendering changes should cover ordered citations within one render call, bibliography sorting, text, HTML, and rendered-tree output when those boundaries are affected.

Run `make rust-lint rust rust-floor` from the repository root.
