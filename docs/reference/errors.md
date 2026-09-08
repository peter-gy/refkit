---
description: Distinguish exceptions, parser diagnostics, tidy warnings, and Polars row or query failures.
---

# Errors and Diagnostics

RefKit separates failed operations, parser diagnostics, and successful formatting warnings. Catch `ParseError` to inspect a failed parse's diagnostics in either binding.

::: code-group

```python [Python]
import refkit as rk

try:
    rk.Library.parse_bibtex("@book{broken")
except rk.ParseError as error:
    print(error.diagnostics[0]["code"])
```

```ts [TypeScript]
import * as rk from "refkit-js";

try {
  rk.Library.parseBibtex("@book{broken");
} catch (error) {
  if (!(error instanceof rk.ParseError)) throw error;
  console.log(error.diagnostics[0]?.code);
}
```

:::

Both examples report `syntax_error`. Diagnostic codes identify the failure category. Messages describe the affected input.

## Error hierarchy

The RefKit hierarchy is shared. `RefkitError` derives from Python `Exception` or JavaScript `Error`.

```text
RefkitError
├── ParseError
├── MissingReferenceError
└── TidyError
    └── TidySyntaxError
```

Argument validation also uses built-in exceptions. Python uses `TypeError` for invalid types and `ValueError` for invalid values. JavaScript uses `TypeError` and `RangeError`, respectively. Lookup methods have the specific behavior in the tables.

## Normalized parsing and lookup

| Operation | Python | TypeScript |
| --- | --- | --- |
| Read a bibliography file | `Library.read`: `RefkitError` for file reads or unsupported extensions, `ParseError` for parser failure. | `readLibrary` from `refkit-js/node`: same RefKit exceptions. |
| Parse BibTeX | `Library.parse_bibtex`: `ParseError` when the recovery policy cannot produce a library. | `Library.parseBibtex`: same failure. |
| Parse [Hayagriva YAML](https://github.com/typst/hayagriva/blob/main/docs/file-format.md), a structured bibliography format | `Library.parse_yaml`: `ParseError` for invalid source. | `Library.parseYaml`: same failure. |
| Look up one citation key | `Library[key]` raises `KeyError`. `Library.get` returns `None` when missing. | `Library.get` returns `null` when missing. |
| Look up several keys | `Library.get_many` raises `KeyError` for a missing key. | `Library.getMany` raises `MissingReferenceError`. |
| Select entries | `Library.select` raises `ValueError` for invalid syntax. | `Library.select` raises `RangeError`. |
| Project entries | `Library.project` raises `TypeError` for invalid collection arguments, `ValueError` for an unknown field, `KeyError` for an absent requested key. | `Library.project` raises `TypeError`, `RangeError`, or `MissingReferenceError`, respectively. |

Report recovery keeps recoverable entries and `Diagnostic` records. Each diagnostic includes a code, severity, recovery action, optional source span, entry key, field, and message. `ParseError.diagnostics` exposes the same shape when parsing fails. YAML failures include `yaml_parse_error` or `resource_limit` diagnostics. See [Data Shapes](/reference/data-shapes).

## Styles and rendering

[CSL](https://citationstyles.org/) (Citation Style Language) defines citation and bibliography formatting in XML style files.

| Failure | Python | TypeScript |
| --- | --- | --- |
| Unknown bundled style or locale | `Style.load` and `Locale.load` raise `ValueError`. | Same methods raise `RangeError`. |
| Invalid or dependent CSL XML, missing/duplicate/cyclic macros, or excessive macro expansion | `Style.from_xml` raises `ValueError`. | `Style.fromXml` raises `RangeError`. |
| Style file read failure | `Style.from_path` raises `RefkitError`. Invalid XML raises `ValueError`. | `readStyle` from `refkit-js/node` raises `RefkitError`. Invalid XML raises `RangeError`. |
| Invalid citation items or an empty group | `CitationGroup` raises `TypeError` for invalid items, `ValueError` for an empty group. | `CitationGroup` raises `TypeError` or `RangeError`, respectively. |
| Unnamed or invalid citation input | `Document.render` and `Document.cited_bibliography` raise `TypeError`. | `Document.render` and `Document.citedBibliography` raise `TypeError`. |
| Duplicate citation IDs or invalid locator labels | Render methods raise `ValueError`. | Render methods raise `RangeError`. |
| Missing citation key or renderer failure | Render methods raise `MissingReferenceError` or `RefkitError`, respectively. | Same exceptions. |
| Unknown result ID | `RenderedDocument[id]` raises `KeyError`. | `RenderedDocument.get(id)` raises `MissingReferenceError`. |

## Raw BibTeX

| Failure | Python | TypeScript |
| --- | --- | --- |
| Missing raw entry or field | Direct indexing raises `KeyError`. `get_unique` returns `None`. | `getUnique` returns `null`. |
| Ambiguous raw entry or field | Direct indexing and `get_unique` raise `RefkitError`. | `getUnique` raises `RefkitError`. |
| Unsafe field replacement | Assigning `BibField.value` raises `ValueError`. | Assigning `BibField.value` raises `RangeError`. |
| Malformed raw block during formatting | `BibDocument.tidy` raises `TidySyntaxError`. | Same exception. |

Python `BibDocument.write` raises `RefkitError` when the destination cannot be written. JavaScript `BibDocument.toBibtex()` returns a string for the host application's file API. Node `tidyFile` wraps destination write failures in `RefkitError`.

## Formatting

`TidySyntaxError` exposes these properties in both bindings:

| Property | Meaning |
| --- | --- |
| `line` | One-based line number. |
| `column` | One-based character column. |
| `byte` | Zero-based UTF-8 byte offset. |
| `character` | First character of the failing block, or `None` / `null` when unavailable. |
| `message` | Parser message without the location prefix. |

`TidyError` also covers key-template, name-processing, ambiguous reference-rewrite, and cyclic reference-rewrite failures. A successful `TidyResult` can contain structured `TidyWarning` values for missing keys and duplicate entries. [Tidy Options](/reference/tidy-options) lists the accepted options and defaults.

## Polars failures

Value expressions turn row-local input, parse, missing-key, and render failures into null. Report expressions preserve parser, renderer, or formatter details in structs.

Invalid static options can raise during expression construction. Invalid dtypes, projection fields, styles, output-name collisions, and broadcasting lengths raise from Polars when the query executes.

Read [Troubleshooting](/troubleshooting) for recovery steps for common failures.
