---
description: Look up every TidyOptions argument, default, merge strategy, duplicate rule, and key-template behavior.
---

# Tidy Options

`TidyOptions` configures canonical BibTeX formatting. Python accepts keyword arguments. TypeScript accepts an options object.

::: code-group

```python [Python]
import refkit as rk

options = rk.TidyOptions(sort_fields=True, wrap=88)
result = rk.tidy_bibtex("@book{doe2024, title={Example}, year={2024}}", options=options)
print(result.count)  # 1
```

```ts [TypeScript]
import * as rk from "refkit-js";
import type { TidyOptions } from "refkit-js";

const options: TidyOptions = { sortFields: true, wrap: 88 };
const result = rk.tidyBibtex("@book{doe2024, title={Example}, year={2024}}", { options });
console.log(result.count); // 1
```

:::

## Options

Defaults use `true`, `false`, and `null`, corresponding to Python's `True`, `False`, and `None`. String-list options accept Python iterables or TypeScript arrays.

| Python | TypeScript | Default | Behavior |
| --- | --- | --- | --- |
| `space` | `space` | `2` | Spaces used for field indentation. |
| `tab` | `tab` | `false` | Indent fields with a tab. |
| `align` | `align` | `14` | Align values at a column. `false` or `null` disables alignment. `true` uses 14. |
| `blank_lines` | `blankLines` | `false` | Insert a blank line between entries. |
| `trailing_commas` | `trailingCommas` | `false` | Add a comma after the final field. |
| `wrap` | `wrap` | `null` | Wrap long values. `true` uses 80 columns. An integer selects the width. |
| `sort` | `sort` | `null` | Sort entries. `true` sorts by key. A list supplies sort fields. Prefix a field with `-` for descending order. |
| `sort_fields` | `sortFields` | `null` | Sort fields. `true` uses the canonical order. A list supplies the order. |
| `omit` | `omit` | `null` | Omit the named fields from output. |
| `curly` | `curly` | `false` | Render non-month values with curly-brace delimiters. |
| `numeric` | `numeric` | `false` | Render positive, nonzero digit strings without delimiters. |
| `months` | `months` | `false` | Normalize month names to BibTeX abbreviations. |
| `strip_enclosing_braces` | `stripEnclosingBraces` | `false` | Remove one redundant brace pair around a complete value. |
| `drop_all_caps` | `dropAllCaps` | `false` | Title-case a value that contains no lowercase letters while preserving Roman numerals. |
| `escape` | `escape` | `true` | Escape supported Unicode and LaTeX-sensitive text in non-verbatim fields while preserving commands and math spans. |
| `encode_urls` | `encodeUrls` | `false` | Convert underscores in URL fields to `\\%5F`. |
| `remove_empty_fields` | `removeEmptyFields` | `false` | Drop fields whose value is empty. |
| `remove_duplicate_fields` | `removeDuplicateFields` | `true` | Keep the first field when a field name repeats in one entry. |
| `max_authors` | `maxAuthors` | `null` | Keep at most this many authors and append `and others` when truncated. |
| `lowercase` | `lowercase` | `true` | Lowercase entry types and field names. |
| `enclosing_braces` | `enclosingBraces` | `null` | Add protective braces inside selected fields. `true` selects `title`. |
| `remove_braces` | `removeBraces` | `null` | Remove protective braces inside selected fields. `true` selects `title`. |
| `strip_comments` | `stripComments` | `false` | Remove source comments from output. |
| `tidy_comments` | `tidyComments` | `true` | Normalize comment layout. |
| `generate_keys` | `generateKeys` | `null` | Generate citation keys. `true` uses the built-in template. A string supplies a template. |
| `duplicates` | `duplicates` | `null` | Report entries matched by selected duplicate rules. |
| `merge` | `merge` | `null` | Merge matched duplicate entries with the selected strategy. |

## Duplicates and merging

Duplicate rules are `doi`, `key`, `abstract`, and `citation`. When duplicate detection or merging is enabled, duplicate-key matches are included in the warnings and are never merged implicitly.

Merge strategies are:

| Strategy | Result |
| --- | --- |
| `first` | Keep the first matched entry. |
| `last` | Keep the last matched entry. |
| `combine` | Keep existing fields and add fields missing from the retained entry. |
| `overwrite` | Add missing fields and replace matching fields with values from the later entry. |

When `merge` is set and `duplicates` is omitted, RefKit matches DOI, citation, and abstract values for merging. It still reports duplicate keys without merging them.

`TidyResult.count` reports the number of input entries. Merging can produce fewer output entries while leaving `count` unchanged.

## Default output contract

Default formatting also normalizes page ranges, converts line endings to LF, and ends output with a newline.

Concatenated `#` expressions preserve their atoms and delimiter modes. Value transforms such as month normalization, escaping, brace changes, numeric output, author truncation, and wrapping do not rewrite those expressions.

## Key templates

Set Python `generate_keys=True` or TypeScript `generateKeys: true` to use:

```text
[auth:required:lower][year:required][veryshorttitle:lower][duplicateNumber]
```

| Marker | Result |
| --- | --- |
| `auth` | First author's surname. |
| `authEtAl` | First two surnames, followed by `EtAl` when there are more authors. |
| `authors` | Every author surname. |
| `authors2` | First two surnames, followed by `EtAl` when truncated. Replace `2` with the desired count. |
| `veryshorttitle` | First title word after removing common function words. |
| `shorttitle` | First three title words after removing common function words. |
| `title` | Capitalized title words. |
| `fulltitle` | All title words with their source capitalization. |
| `year` | Digits from the year field. |
| Uppercase field name, such as `DOI` | Words from the named field. |
| `duplicateLetter` | Letter suffix when multiple entries generate the same key. |
| `duplicateNumber` | Numeric suffix when multiple entries generate the same key. |

Markers use square brackets. Append modifiers with `:`, in execution order: `required`, `lower`, `upper`, and `capitalize`.

::: code-group

```python [Python]
key_options = rk.TidyOptions(generate_keys="[auth:lower][year:required]")
key_result = rk.tidy_bibtex(
    "@book{old, author={Doe, Jane}, title={Example}, year={2024}}",
    options=key_options,
)
print(key_result.renames[0]["new_key"])  # doe2024
```

```ts [TypeScript]
const keyOptions: TidyOptions = { generateKeys: "[auth:lower][year:required]" };
const keyResult = rk.tidyBibtex(
  "@book{old, author={Doe, Jane}, title={Example}, year={2024}}",
  { options: keyOptions },
);
console.log(keyResult.renames[0]?.newKey); // doe2024
```

:::

For `author={Doe, Jane}` and `year={2024}`, this template produces `doe2024`. A duplicate receives a suffix. Missing required source data keeps the entry's original key. Literal text outside markers is retained subject to citation-key character validation.

The formatter assigns globally unique final keys, including entries that retain their original key. It updates `crossref` and `xdata` references, then sorts by the emitted keys when key sorting is enabled. A source reference that could identify multiple final entries, or a transformation that creates a reference cycle, raises `TidyError` before output is returned.

`TidyResult.renames` records the source occurrence, old key, and final key for changed identities, including entries merged into another entry. Update citation keys in external documents from this report.

Malformed source raises `TidySyntaxError`. Invalid option types raise `TypeError`. Invalid duplicate rules and merge strategies raise Python `ValueError` or JavaScript `RangeError`. Invalid key templates raise `TidyError`. See [Errors and Diagnostics](/reference/errors) for location properties and warnings.
