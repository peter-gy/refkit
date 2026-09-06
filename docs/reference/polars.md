---
description: Look up all 21 polars-refkit expressions, dtypes, defaults, broadcasting rules, and failure boundaries.
---

# Polars Expressions

`polars-refkit` exports 21 expression builders. Importing `polars_refkit` also registers each builder under `pl.Expr.refkit`.

## Shared arguments

`bibtex_col`, `key_col`, and `keys_col` accept a column name string or `pl.Expr`. A plain string names a column. Use `pl.lit(...)` for literal values.

Rendering expressions share:

```python
style = "apa"
locale = "en-US"
recovery = "error"
```

`style` names a bundled style and is loaded case-insensitively. An unknown style aborts the query with `ComputeError`. The Polars interface accepts bundled style names rather than `Style` objects, paths, or CSL XML.

`locale` is forwarded to the renderer. Use a known bundled locale code. An empty string selects no explicit locale. Raw locale strings are not validated before rendering.

## Parse and inspect

| Expression | Output dtype | Default output name | Failure result |
| --- | --- | --- | --- |
| `entry_count(bibtex_col)` | `UInt32` | `entry_count` | Null. |
| `can_parse(bibtex_col)` | `Boolean` | `can_parse` | `False`. |
| `has_diagnostics(bibtex_col)` | `Boolean` | `has_diagnostics` | Reflects parser messages. |
| `keys(bibtex_col)` | `List[String]` | `keys` | Null. |
| `entries(bibtex_col, fields=None)` | `List[Struct]` | `entries` | Null. |
| `diagnostics(bibtex_col)` | `List[String]` | `diagnostics` | Parser messages. |
| `parse_report(bibtex_col)` | report struct | `parse_report` | Structured status. |

Every operation accepts `recovery="error"` or `recovery="report"`.

`entries` defaults to `key`, `title`, `doi`, and `volume`. Supported fields are `key`, `entry_type`, `type`, `title`, `date`, `doi`, and `volume`. Unknown or repeated fields abort the query. An empty field list returns one empty struct per normalized entry.

`parse_report` returns `ok`, `entry_count`, `keys`, and `diagnostics`. For a null source, the current result is a non-null struct whose four fields are null. Other report-like expressions use an outer null for null source input.

## Render a Citation

```python
cite(bibtex_col, key_col, *, style="apa", locale="en-US", recovery="error")
cite_html(bibtex_col, key_col, *, style="apa", locale="en-US", recovery="error")
cite_rendered(bibtex_col, key_col, *, style="apa", locale="en-US", recovery="error")
```

| Expression | Output dtype | Default output name |
| --- | --- | --- |
| `cite` | `String` | `cite` |
| `cite_html` | `String` | `cite_html` |
| `cite_rendered` | `Struct[text: String, html: String]` | `cite_rendered` |

## Render each key

```python
cite_each(bibtex_col, keys_col, *, style="apa", locale="en-US", recovery="error")
cite_each_html(bibtex_col, keys_col, *, style="apa", locale="en-US", recovery="error")
cite_each_rendered(bibtex_col, keys_col, *, style="apa", locale="en-US", recovery="error")
```

`keys_col` has dtype `List[String]`. The result contains one separate citation per key in order. Output dtypes are `List[String]`, `List[String]`, and `List[Struct[text: String, html: String]]`.

An empty key list returns an empty list.

## Render a Citation Group

```python
cite_group(bibtex_col, keys_col, *, style="apa", locale="en-US", recovery="error")
cite_group_html(bibtex_col, keys_col, *, style="apa", locale="en-US", recovery="error")
cite_group_rendered(bibtex_col, keys_col, *, style="apa", locale="en-US", recovery="error")
```

The ordered keys render as one citation group. Output dtypes are `String`, `String`, and `Struct[text: String, html: String]`.

An empty key list returns an empty string or `{text: "", html: ""}`.

## Render the full bibliography

```python
full_bibliography_text(bibtex_col, *, style="apa", locale="en-US", recovery="error")
full_bibliography_html(bibtex_col, *, style="apa", locale="en-US", recovery="error")
full_bibliography_rendered(bibtex_col, *, style="apa", locale="en-US", recovery="error")
```

Each operation renders every normalized entry in its bibliography source row. Polars rows have no cited-bibliography state.

## Format BibTeX

`tidy_bibtex(bibtex_col, **options)` returns a string column. `tidy_bibtex_report(bibtex_col, **options)` returns `ok`, `bibtex`, `count`, `warnings`, and `error`.

Both builders accept the options in [Tidy Options](/reference/tidy-options). Omitted keyword arguments use the core defaults.

## Broadcasting

Two-input citation families accept:

- Equal input lengths.
- One bibliography source and many key values.
- Many bibliography sources and one key value.

Other length combinations raise `ComputeError`. A singleton valid source is parsed once inside the expression. Separate expressions keep separate parse work.

## Failure boundaries

Row-local nulls cover null input, parse failure, missing citation keys, null key-list items, and render failure for value expressions. Parser diagnostics explain parse failures. Render failures have no report expression.

Query-wide failures include invalid input dtype, unsupported projection fields, repeated projection fields, unknown styles, and non-broadcastable lengths. Static recovery and tidy option validation can raise before a query runs.

Every builder supplies a default output name. Alias repeated operations with the same name:

```python
frame.select(
    pl.col("bibtex").refkit.cite("primary_key").alias("primary"),
    pl.col("bibtex").refkit.cite("secondary_key").alias("secondary"),
)
```
