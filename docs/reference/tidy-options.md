---
description: Look up every TidyOptions argument, default, merge strategy, duplicate rule, and key-template behavior.
---

# Tidy Options

`TidyOptions` configures canonical BibTeX formatting. Every argument is keyword-only.

```python
options = rk.TidyOptions(sort_fields=True, wrap=88)
```

## Layout and ordering

| Option | Default | Behavior |
| --- | --- | --- |
| `space` | `2` | Spaces used for field indentation. |
| `tab` | `False` | Indent fields with a tab. |
| `align` | `14` | Align values at a column. `False` or `None` disables alignment. `True` uses 14. |
| `blank_lines` | `False` | Insert a blank line between entries. |
| `trailing_commas` | `False` | Add a comma after the final field. |
| `wrap` | `None` | Wrap long values. `True` uses 80 columns. An integer selects the width. |
| `sort` | `None` | Sort entries. `True` sorts by key. An iterable supplies sort fields. Prefix a field with `-` for descending order. |
| `sort_fields` | `None` | Sort fields. `True` uses the canonical order. An iterable supplies the order. |

## Fields and values

| Option | Default | Behavior |
| --- | --- | --- |
| `omit` | `None` | Omit the named fields from output. |
| `curly` | `False` | Render non-month values with curly-brace delimiters. |
| `numeric` | `False` | Render positive, nonzero digit strings without delimiters. |
| `months` | `False` | Normalize month names to BibTeX abbreviations. |
| `strip_enclosing_braces` | `False` | Remove one redundant brace pair around a complete value. |
| `drop_all_caps` | `False` | Title-case a value that contains no lowercase letters while preserving Roman numerals. |
| `escape` | `True` | Escape supported Unicode and LaTeX-sensitive text in non-verbatim fields while preserving commands and math spans. |
| `encode_urls` | `False` | Convert underscores in URL fields to `\\%5F`. |
| `remove_empty_fields` | `False` | Drop fields whose value is empty. |
| `remove_duplicate_fields` | `True` | Keep the first field when a field name repeats in one entry. |
| `max_authors` | `None` | Keep at most this many authors and append `and others` when truncated. |
| `lowercase` | `True` | Lowercase entry types and field names. |
| `enclosing_braces` | `None` | Add protective braces inside selected fields. `True` selects `title`. |
| `remove_braces` | `None` | Remove protective braces inside selected fields. `True` selects `title`. |

## Comments

| Option | Default | Behavior |
| --- | --- | --- |
| `strip_comments` | `False` | Remove source comments from output. |
| `tidy_comments` | `True` | Normalize comment layout. |

## Keys and duplicates

| Option | Default | Behavior |
| --- | --- | --- |
| `generate_keys` | `None` | Generate citation keys. `True` uses the built-in template. A string supplies a template. |
| `duplicates` | `None` | Report entries matched by selected duplicate rules. |
| `merge` | `None` | Merge matched duplicate entries with the selected strategy. |

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

`generate_keys=True` uses:

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

```python
options = rk.TidyOptions(generate_keys="[auth:lower][year:required]")
```

For `author={Doe, Jane}` and `year={2024}`, this template produces `doe2024`. A duplicate receives a suffix. Missing required source data keeps the entry's original key. Literal text outside markers is retained subject to citation-key character validation.

The formatter assigns globally unique final keys, including entries that retain their original key. It updates `crossref` and `xdata` references, then sorts by the emitted keys when key sorting is enabled. A source reference that could identify multiple final entries, or a transformation that creates a reference cycle, raises `TidyError` before output is returned.

`TidyResult.renames` records the source occurrence, old key, and final key for changed identities, including entries merged into another entry. Update citation keys in external documents from this report.

Malformed source raises `TidySyntaxError`. Invalid option types, duplicate rules, merge strategies, and key templates raise `TypeError`, `ValueError`, or `TidyError` before a formatted result is returned.
