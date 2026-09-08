---
description: Filter normalized Library entries by type, fields, alternatives, and parent structure.
---

# Selectors

`Library.select(selector)` filters normalized entries with the [Hayagriva selector language](https://github.com/typst/hayagriva/blob/main/docs/selectors.md), a grammar for matching entry types, fields, and parent structure.

::: code-group

```python [Python]
import refkit as rk

library = rk.Library.parse_bibtex("@book{doe2024, title={Example}, year={2024}}")
selected = library.select("book[title,date]")
print([entry.key for entry in selected])  # ['doe2024']
```

```ts [TypeScript]
import * as rk from "refkit-js";

const library = rk.Library.parseBibtex("@book{doe2024, title={Example}, year={2024}}");
const selected = library.select("book[title,date]");
console.log(selected.map((entry) => entry.key)); // ["doe2024"]
```

:::

## Grammar

| Selector | Matches |
| --- | --- |
| `article` | Entries of the named type. |
| `*` | Every entry. |
| `article[date]` | Articles with a date. |
| `article[author,title,date]` | Articles with a value for every listed field. |
| `article > periodical[volume]` | Articles with a periodical parent that has a volume. |
| `article > (conference & video)` | Articles with both matching parent types. |
| `book \| article` | Either entry type. |
| `!book` | Entries that fail the book selector. |

String selectors are case-insensitive. Entry types and fields use Hayagriva's normalized bibliography model. For example, a BibTeX `year` becomes part of `date`.

`>` requires a matching parent and can be chained for deeper relationships. `&` requires multiple matching parents on the right side of `>`. `|` selects alternatives, `!` negates the next selector, and parentheses group expressions.

`select` returns the top-level `Entry` objects that match. Selector bindings participate in matching. The returned records are the matching entries.

An invalid selector raises Python `ValueError` or JavaScript `RangeError` before any result list is returned.
