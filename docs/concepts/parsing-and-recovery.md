---
description: Choose exact normalized parsing, report recovery, raw failed blocks, or Polars row diagnostics.
---

# Parsing and Recovery

Parsing can either require an exact normalized result or retain recoverable entries with diagnostics. Raw parsing uses a separate block model so malformed source remains inspectable.

## Require an exact normalized result

`Library.read` and `Library.parse_bibtex` default to `recovery="error"`. A BibTeX problem that produces parser diagnostics raises `ParseError`.

```python
import refkit as rk

library = rk.Library.parse_bibtex(source)
```

Use this policy when later code should run only on an exact normalized parse.

## Retain recoverable entries

`recovery="report"` returns the entries recovered by the parser and records structured diagnostics in `Library.diagnostics`:

```python
library = rk.Library.parse_bibtex(source, recovery="report")

for diagnostic in library.diagnostics:
    print(diagnostic["code"], diagnostic["span"], diagnostic["message"])
```

A non-empty source that yields no entries and has recovery diagnostics raises `ParseError`. Empty or comment-only input can produce an empty library.

## Input limits

Bibliography source is limited to 16 MiB of UTF-8 text. BibTeX parsing also bounds recursive work:

| Work | Limit |
| --- | --- |
| Value nesting | 64 levels |
| Macro or bibliography-reference depth | 64 levels |
| Expanded bibliography data | 16 MiB |
| Dependency traversal | 100,000 steps |
| Report recovery | 128 changes |

A limit failure produces a `resource_limit` diagnostic. Report recovery returns that failure. Split large independent bibliographies or simplify deeply nested values before retrying. Tidy applies the same source-size and value-nesting guards before recursive formatting.

## Inspect malformed raw blocks

`BibDocument.parse` scans the source into blocks even when individual blocks are malformed:

```python
raw = rk.BibDocument.parse(source)

for block in raw.failed_blocks:
    print(block["span"], block["error"], block["raw"])
```

Each failed block carries its raw text, parser error, and two-item byte span. `BibDocument.blocks` keeps failed blocks in their original source order.

## Distinguish diagnostics, warnings, and errors

| Term | Produced by | Meaning |
| --- | --- | --- |
| Diagnostic | Normalized BibTeX parsing | A record identifying the affected source, action, and parser message. |
| Warning | BibTeX formatting | A structured, non-fatal missing-key or duplicate-entry result. |
| Error | Parsing, formatting, rendering, or I/O | The requested operation could not produce its contract. |

Polars uses the same recovery policies per bibliography source row. Read [Process Polars Columns](/guides/polars) for row results and query-level failures.
