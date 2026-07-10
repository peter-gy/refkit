# Portable Core

`refkit-core` owns bibliography behavior that every interface can reuse.

- Return typed Rust records from public core operations. Convert them into Python, Polars, or JSON shapes in adapters.
- Keep Hayagriva and BibLaTeX types behind the core API where a stable RefKit type can express the contract.
- Preserve source spans, UTF-8 boundaries, unrelated valid entries, and deterministic diagnostic order during recovery.
- Keep raw BibTeX parsing and tidy formatting on one syntax model.
- Exercise changes with `make rust-lint rust rust-floor` from the repository root.

Rendering changes should cover document history, bibliography sorting, text, HTML, and rendered-tree output when those boundaries are affected.
