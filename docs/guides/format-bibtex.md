---
description: Canonically format BibTeX strings, raw documents, or files and inspect structured warnings.
---

# Format BibTeX

`tidy_bibtex` parses and formats a BibTeX string. It returns the formatted source, entry count, and structured warnings.

## Format a string

```python
import refkit as rk

result = rk.tidy_bibtex("@ARTICLE {doe2024, pages={6-13}, year={2024},}\n")

print(result.bibtex)
print(result.count)
```

Expected formatted source:

```bibtex
@article{doe2024,
  pages         = {6--13},
  year          = {2024}
}
```

The default formatter lowercases entry and field names, aligns values at column 14, uses two spaces for indentation, normalizes page ranges, escapes supported Unicode and LaTeX-sensitive text, tidies comments, and keeps the first occurrence of each field name.

## Choose formatting options

```python
options = rk.TidyOptions(
    sort_fields=True,
    wrap=88,
    trailing_commas=True,
)
result = rk.tidy_bibtex(source, options=options)
```

Several options accept a boolean shorthand or an explicit value. `wrap=True` uses 80 columns, while `wrap=88` selects 88. `sort_fields=True` uses the canonical field order, while an iterable supplies a custom order.

Read [Tidy Options](/reference/tidy-options) for every argument, default, and accepted form.

## Format a raw document

```python
document = rk.BibDocument.read("references.bib")
result = document.tidy(options=options)
```

`BibDocument.tidy` formats the current in-memory document state. It returns `TidyResult` and leaves file output to the caller.

## Read and optionally write a file

```python
result = rk.tidy_file(
    "references.bib",
    output="references.formatted.bib",
    options=options,
)
```

When `output` is omitted, `tidy_file` returns the result without writing a file.

## Inspect warnings

```python
for warning in result.warnings:
    print(warning.code, warning.rule, warning.message)
```

`missing_key` warns about an entry without a citation key. `duplicate_entry` includes the matching duplicate rule.

Formatting a malformed block raises `TidySyntaxError`. Its `line`, `column`, `byte`, `character`, and `message` properties locate the parser failure.
