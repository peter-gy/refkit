---
description: Choose exact normalized parsing, report recovery, raw failed blocks, or Polars row diagnostics.
---

# Parsing and Recovery

Parsing can require an exact normalized result or retain recoverable entries with diagnostics. Raw parsing uses a separate block model so malformed source remains inspectable. [BibTeX](https://www.bibtex.org/) stores bibliography entries as text and supports named abbreviations for field values.

## Require an exact normalized result

Normalized BibTeX parsing defaults to the `error` recovery policy. A parser diagnostic raises `ParseError`. The abbreviation `unknown_title` has no definition in this source:

::: code-group

```python [Python]
import refkit as rk

source = "@book{good,title={Known title}}\n@book{bad,title=unknown_title}"
try:
    rk.Library.parse_bibtex(source)
except rk.ParseError as error:
    print(error.diagnostics[0]["code"])
```

```ts [TypeScript]
import * as rk from "refkit-js";

const source = "@book{good,title={Known title}}\n@book{bad,title=unknown_title}";
try {
  rk.Library.parseBibtex(source);
} catch (error) {
  if (!(error instanceof rk.ParseError)) throw error;
  console.log(error.diagnostics[0]!.code);
}
```

:::

Both print `unknown_abbreviation`. TypeScript examples run in Node.js. For browsers, complete [browser initialization](/guides/browser#initialize-the-module) first.

## Retain recoverable entries

The `report` policy retains recovered entries and records diagnostics. Using the same `source`, the parser treats `unknown_title` as literal text:

::: code-group

```python [Python]
library = rk.Library.parse_bibtex(source, recovery="report")
print(library["bad"].title)
for diagnostic in library.diagnostics:
    print(diagnostic["code"], diagnostic["action"])
```

```ts [TypeScript]
const library = rk.Library.parseBibtex(source, { recovery: "report" });
console.log(library.get("bad")!.title);
for (const diagnostic of library.diagnostics) {
  console.log(diagnostic.code, diagnostic.action);
}
```

:::

Both print `unknown_title`, followed by `unknown_abbreviation literalized`. Each diagnostic also carries a message and an optional byte span into the UTF-8 source. Byte offsets can differ from character indices for non-ASCII text.

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

A limit failure produces a `resource_limit` diagnostic and raises `ParseError` under both recovery policies. Split large independent bibliographies or simplify deeply nested values before retrying. Tidy applies the same source-size and value-nesting guards before recursive formatting.

## Inspect malformed raw blocks

`BibDocument.parse` scans the source into blocks even when individual blocks are malformed. Using the `rk` import, inspect an entry with an unclosed value:

::: code-group

```python [Python]
raw = rk.BibDocument.parse("@book{broken,title={Unclosed")
for block in raw.failed_blocks:
    print(block["span"], block["raw"])
```

```ts [TypeScript]
const raw = rk.BibDocument.parse("@book{broken,title={Unclosed");
for (const block of raw.failedBlocks) {
  console.log(block.span, block.raw);
}
```

:::

The failed block retains `@book{broken,title={Unclosed` and its byte span `[0, 28]`. Each failed block also carries the parser error. The document's block list keeps failed blocks in source order.

## Distinguish diagnostics, warnings, and errors

| Term | Produced by | Meaning |
| --- | --- | --- |
| Diagnostic | Normalized BibTeX parsing | A record identifying the affected source, action, and parser message. |
| Warning | BibTeX formatting | A structured, non-fatal missing-key or duplicate-entry result. |
| Error | Parsing, formatting, rendering, or I/O | The requested operation could not produce its contract. |

[Polars](https://docs.pola.rs/), a DataFrame query engine, applies the same recovery policies per bibliography source row. Read [Process Polars Columns](/guides/polars) for row results and query-level failures.
