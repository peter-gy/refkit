---
description: Look up bibliography expressions, output formats, dtypes, broadcasting, and row reports.
---

# Polars Expressions

Importing `polars_refkit` registers the `pl.Expr.refkit` namespace. Every expression also has a top-level builder with the bibliography column as its first argument.

## Shared inputs and options

`bibtex_col`, `key_col`, and `keys_col` accept a column name or `pl.Expr`. Use `pl.lit(...)` for literal bibliography text or keys.

Parsing accepts `recovery="error"` or `recovery="report"`. Rendering also accepts `style="apa"` and `locale="en-US"`. `style` selects a bundled style case-insensitively. `locale` is forwarded to the renderer, and an empty string selects no explicit locale.

Rendering accepts `output="text"`, `"html"`, or `"rendered"`. The output choice is validated when constructing the expression and determines its dtype. A rendered value is `Struct[text: String, html: String]`.

## Parse and inspect

| Expression | Output dtype | Parse failure |
| --- | --- | --- |
| `entry_count(bibtex_col, *, recovery="error")` | `UInt32` | Null. |
| `can_parse(bibtex_col, *, recovery="error")` | `Boolean` | `False`. |
| `has_diagnostics(bibtex_col, *, recovery="error")` | `Boolean` | Reflects diagnostic presence. |
| `keys(bibtex_col, *, recovery="error")` | `List[String]` | Null. |
| `entries(bibtex_col, *, fields=None, recovery="error")` | `List[Struct]` | Null. |
| `resolve(bibtex_col)` | `List[ResolvedBibEntry]` | Null. |
| `diagnostics(bibtex_col, *, recovery="error")` | `List[Diagnostic]` | Structured diagnostic records. |
| `parse_report(bibtex_col, *, recovery="error")` | Parse report struct | `ok=False` with diagnostics. |

`entries` defaults to `key`, `title`, `doi`, and `volume`. Supported fields are `key`, `entry_type`, `type`, `title`, `date`, `doi`, and `volume`. An empty field list returns one empty struct per entry. Unknown or repeated fields abort the query.

`parse_report` performs one parse for `ok`, `entry_count`, `keys`, and `diagnostics`. Null source produces a null report. [Data Shapes](/reference/data-shapes) defines report and diagnostic fields.

`resolve` expands string macros and concatenations across every source field,
including custom fields. It preserves literal TeX grouping and escapes. Each
row is independent, and an empty bibliography produces an empty list. See
[resolved entry records](/reference/data-shapes#resolved-entry-records) for the
fixed nested dtype. Use `BibDocument.resolve()` in Python to inspect a failed
row's structured `ParseError` diagnostics.

## Render citations

```python
cite(bibtex_col, key_col, *, style="apa", locale="en-US", recovery="error", output="text")
cite_each(bibtex_col, keys_col, *, style="apa", locale="en-US", recovery="error", output="text")
cite_group(bibtex_col, keys_col, *, style="apa", locale="en-US", recovery="error", output="text")
```

| Operation | Key dtype | Result per row |
| --- | --- | --- |
| `cite` | `String` | One citation. |
| `cite_each` | `List[String]` | A list of citations in input order, sharing citation state within the row. |
| `cite_group` | `List[String]` | One citation containing the ordered group. |

Text and HTML outputs are strings. `cite_each` returns a list of the selected output type. An empty `cite_each` key list returns `[]`. A group must contain at least one key. An empty `cite_group` produces null, and its grouped render report records a `render_error`.

## Render every entry

```python
full_bibliography(bibtex_col, *, style="apa", locale="en-US", recovery="error", output="text")
```

Renders the complete normalized library in each source row. Each row owns independent citation state.

## Inspect a render failure

```python
render_report(bibtex_col, keys_col, *, grouped=False, style="apa", locale="en-US", recovery="error")
```

Accepts a `List[String]` key column and returns `ok`, `citations`, `diagnostics`, `error_code`, and `error`. `grouped=False` renders one ordered citation per key. `grouped=True` renders one grouped citation. Each citation contains text and HTML.

`error_code` is `parse_error`, `missing_key`, or `render_error` on failure. Successful reports have null error fields. Null source or key-list input produces a null report.

## Format BibTeX

```python
tidy_bibtex(bibtex_col, *, options=None)
tidy_bibtex_report(bibtex_col, *, options=None)
```

Pass a dictionary typed as `polars_refkit.TidyOptions`:

```python
import polars_refkit as prk

options: prk.TidyOptions = {"sort_fields": True, "wrap": 88}
formatted = pl.col("bibtex").refkit.tidy_bibtex(options=options)
```

Omitted keys use the core defaults in [Tidy Options](/reference/tidy-options). Optional rules accept their explicit value or the documented boolean shorthand. `None` and `False` disable optional rules.

`tidy_bibtex` returns a string. `tidy_bibtex_report` returns `ok`, `bibtex`, `count`, `warnings`, `renames`, and `error`. Null input produces null output.

## Broadcasting and failures

Two-input expressions accept equal input lengths or a length-one input on either side. A singleton source is parsed once within that expression, including failed parses. Other lengths raise `ComputeError`. Separate expressions parse independently.

Value expressions return null for row-local null input, parse failure, missing citation keys, null key-list items, or render failure. Reports preserve the corresponding detail.

Invalid input dtypes, unknown styles, unsupported or repeated projection fields, and incompatible lengths abort the query. Invalid recovery, output, and tidy options raise during expression construction.

Each expression's default output name is its operation name. Alias repeated operations:

```python
frame.select(
    primary=pl.col("bibtex").refkit.cite("key"),
    html=pl.col("bibtex").refkit.cite("key", output="html"),
)
```
