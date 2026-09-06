---
name: refkit
description: Use RefKit to parse, inspect, render, format, or safely edit BibTeX, BibLaTeX, and Hayagriva bibliography data from Python or Polars.
---

# RefKit

RefKit exposes one portable bibliography core through Python objects and Polars expressions. Work through the public package APIs so notebook code, scripts, and applications share the same parsing, rendering, formatting, and error contracts.

## Choose the data model

- Use `refkit.Library` for normalized entries, diagnostics, selection, projection, citation rendering, and bibliography rendering.
- Use `refkit.BibDocument` for source-order BibTeX blocks, duplicate occurrences, existing-field edits, and preserving writes.
- Use `polars_refkit` when bibliography source already lives in eager or lazy Polars queries.

Do not flatten these models into one generic dictionary workflow. Their ownership and failure behavior differ.

## Working rules

1. Prefer in-memory APIs inside the active Python process. Use `Library.parse_bibtex`, `Library.parse_yaml`, `BibDocument.parse`, and `tidy_bibtex` when source text is already available.
2. Use `recovery="report"` when the task should retain recoverable entries. Always inspect and report `library.diagnostics` before consuming those entries. Use `recovery="error"` when malformed input must stop the operation.
3. Project the fields needed for inspection with `Library.project`. Avoid dumping complete large libraries or render trees into notebook output.
4. Build one `Document` from a prepared `Library`, `Style`, and locale. Pass the complete ordered citation sequence to one `render` call because order can affect numbering, disambiguation, position-sensitive formatting, and the cited bibliography.
5. Choose `Rendered.text`, `Rendered.html`, or `Rendered.tree` at the consumer boundary. Treat HTML as rendered output and tree nodes as structured data.
6. Select duplicate raw entry or field occurrences with `get_all` before editing an existing field. `BibEntry.key` is read-only. Preview `BibDocument.to_bibtex()` before writing a file.
7. Inspect `TidyResult.warnings` before accepting canonical formatting, key generation, duplicate detection, or merges.
8. Compare requested citation keys with `Library.keys()` before a batch render. A missing key aborts the complete render call, so return the requested, missing, and available keys and ask the user to resolve the mismatch.
9. Let RefKit exceptions reach the agent runtime while diagnosing. Catch a specific error only when the workflow has a concrete recovery path.

## Start with a bounded inspection

```python
import refkit as rk

source = "@article{doe2024, title={Fast Citations}, year={2024}}"
library = rk.Library.parse_bibtex(source, recovery="report")
rows = library.project(["key", "entry_type", "title", "date", "doi"])
diagnostics = list(library.diagnostics)
```

Return `rows` and `diagnostics` together when report recovery retains partial input.

Read [references/workflows.md](references/workflows.md) for complete normalized, rendering, raw-edit, formatting, and Polars workflows. Read [references/contracts.md](references/contracts.md) when the task depends on lifecycle, duplicate, error, or output-shape details.

The installed documentation map is available at `https://peter-gy.github.io/refkit/llms.txt`.
