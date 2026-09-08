---
description: Format BibTeX strings, raw documents, or files and inspect warnings and key renames in Python or TypeScript.
---

# Format BibTeX

The formatter normalizes [BibTeX](https://ctan.org/pkg/bibtex) source and returns the formatted text, entry count, structured warnings, and key renames.

## Format a string

::: code-group

```python [Python]
import refkit as rk

source = "@ARTICLE {doe2024, pages={6-13}, year={2024},}\n"
result = rk.tidy_bibtex(source)
print(result.bibtex)
assert result.count == 1
```

```ts [TypeScript]
import * as rk from "refkit-js";

const source = "@ARTICLE {doe2024, pages={6-13}, year={2024},}\n";
const result = rk.tidyBibtex(source);
console.log(result.bibtex);
```

:::

```bibtex
@article{doe2024,
  pages         = {6--13},
  year          = {2024}
}
```

The default formatter lowercases entry and field names, aligns values at column 14, uses two spaces for indentation, normalizes page ranges, escapes supported Unicode and LaTeX-sensitive text, tidies comments, and keeps the first occurrence of each field name.

TypeScript examples run in Node.js. In a browser, [initialize RefKit](/guides/browser#initialize-the-module) before formatting. The remaining examples continue with `source`.

## Choose formatting options

::: code-group

```python [Python]
options = rk.TidyOptions(sort_fields=True, wrap=88, trailing_commas=True)
formatted = rk.tidy_bibtex(source, options=options)
```

```ts [TypeScript]
const options: rk.TidyOptions = {
  sortFields: true,
  wrap: 88,
  trailingCommas: true,
};
const formatted = rk.tidyBibtex(source, { options });
```

:::

Several options accept a boolean shorthand or an explicit value. `wrap=True` / `wrap: true` uses 80 columns, while `88` selects 88. Enabling field sorting uses the canonical order. A list of field names supplies a custom order.

Read [Tidy Options](/reference/tidy-options) for every argument, default, and accepted form.

## Format a raw document

::: code-group

```python [Python]
document = rk.BibDocument.parse(source)
formatted_document = document.tidy(options=options)
```

```ts [TypeScript]
const document = rk.BibDocument.parse(source);
const formattedDocument = document.tidy({ options });
```

:::

`BibDocument.tidy` formats the current in-memory document state. It returns `TidyResult` and leaves file output to the caller.

## Read and optionally write a file

Format an existing `references.bib` and write its result to a separate path:

::: code-group

```python [Python]
formatted_file = rk.tidy_file(
    "references.bib",
    output="references.formatted.bib",
    options=options,
)
```

```ts [TypeScript]
import { tidyFile } from "refkit-js/node";

const formattedFile = await tidyFile("references.bib", {
  output: "references.formatted.bib",
  options,
});
```

:::

Omit `output` to return the result and leave the filesystem unchanged.

## Inspect warnings

::: code-group

```python [Python]
warnings_result = rk.tidy_bibtex(
    "@book{first,doi={10.1/same}}\n@book{second,doi={10.1/same}}",
    options=rk.TidyOptions(duplicates=["doi"]),
)
for warning in warnings_result.warnings:
    print(warning.code, warning.rule)
```

```ts [TypeScript]
const warningsResult = rk.tidyBibtex(
  "@book{first,doi={10.1/same}}\n@book{second,doi={10.1/same}}",
  { options: { duplicates: ["doi"] } },
);
for (const warning of warningsResult.warnings) {
  console.log(warning.code, warning.rule);
}
```

:::

Both examples print `duplicate_entry doi`. Each warning also includes a `message` explaining the affected entries. `missing_key` warns about an entry lacking a citation key. `duplicate_entry` includes the matching duplicate rule.

Formatting a malformed block raises `TidySyntaxError`. Its `line`, `column`, `byte`, `character`, and `message` properties locate the parser failure.

## Generate keys and update references

::: code-group

```python [Python]
renamed = rk.tidy_bibtex(
    "@book{draft, author={Doe, Jane}, title={Fast Citations}, year={2024}}",
    options=rk.TidyOptions(generate_keys=True, sort=True),
)
for rename in renamed.renames:
    print(rename["entry_id"], rename["old_key"], rename["new_key"])
```

```ts [TypeScript]
const renamed = rk.tidyBibtex(
  "@book{draft, author={Doe, Jane}, title={Fast Citations}, year={2024}}",
  { options: { generateKeys: true, sort: true } },
);
for (const rename of renamed.renames) {
  console.log(rename.entryId, rename.oldKey, rename.newKey);
}
```

:::

The rename record maps `draft` to `doe2024fast`. Key generation and merging share one plan. Final keys are unique, bibliography `crossref` and `xdata` fields point to those keys, and key sorting uses the final names. The rename report lets an application update citations in other files. Inspect the report before writing a bibliography used by an existing document.
