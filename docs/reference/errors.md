---
description: Distinguish Python exceptions, parser diagnostics, tidy warnings, and Polars row or query failures.
---

# Errors and Diagnostics

RefKit separates failed operations, parser diagnostics, and successful formatting warnings.

## Python error hierarchy

```text
Exception
└── RefkitError
    ├── ParseError
    ├── MissingReferenceError
    └── TidyError
        └── TidySyntaxError
```

Argument shape and lookup failures can use built-in `TypeError`, `ValueError`, and `KeyError`.

## Normalized parsing and lookup

| Operation | Failure |
| --- | --- |
| `Library.read` | `RefkitError` for file reads or unsupported extensions, and `ParseError` for parser failure. |
| `Library.parse_bibtex` | `ParseError` when the selected recovery policy cannot produce a library. |
| `Library.parse_yaml` | `ParseError` for invalid Hayagriva YAML. |
| `Library[key]`, `get_many` | `KeyError` for a missing citation key. |
| `Library.select` | `ValueError` for invalid selector syntax. |
| `Library.project` | `TypeError` for invalid collection arguments, `ValueError` for an unknown field, and `KeyError` for an absent requested key. |

Report recovery keeps recoverable entries and `Diagnostic` dictionaries. Each diagnostic includes a code, severity, recovery action, optional source span, entry key, field, and message. `ParseError` also exposes these records through `error.diagnostics`. YAML failures include `yaml_parse_error` or `resource_limit` diagnostics. See [Data Shapes](/reference/data-shapes).

## Styles and rendering

| Operation | Failure |
| --- | --- |
| `Style.load` | `ValueError` for an unknown bundled style. |
| `Style.from_xml` | `ValueError` for invalid or dependent CSL XML, missing/duplicate/cyclic macros, or excessive macro expansion. |
| `Style.from_path` | `RefkitError` for file reads and `ValueError` for invalid style XML. |
| `Locale.load` | `ValueError` for an unknown bundled locale code. |
| `CitationGroup` | `TypeError` for invalid item input and `ValueError` for an empty group. |
| `Document.render`, `Document.cited_bibliography` | `TypeError` for unnamed or invalid citation input, `ValueError` for duplicate IDs or locator labels, `MissingReferenceError` for an absent key, and `RefkitError` for renderer failure. |
| `RenderedDocument[id]` | `KeyError` for an unknown result ID. |

## Raw BibTeX

| Operation | Failure |
| --- | --- |
| Direct entry or field lookup | `KeyError` when missing and `RefkitError` when duplicate occurrences make the key ambiguous. |
| `BibField.value = value` | `ValueError` when the replacement cannot be represented safely. |
| `BibDocument.write` | `RefkitError` when the destination cannot be written. |
| `BibDocument.tidy` | `TidySyntaxError` when the current raw state contains a malformed block. |

## Formatting

`TidySyntaxError` exposes:

| Property | Meaning |
| --- | --- |
| `line` | One-based line number. |
| `column` | One-based character column. |
| `byte` | Zero-based UTF-8 byte offset. |
| `character` | First character of the failing block when available. |
| `message` | Parser message without the location prefix. |

`TidyError` also covers key-template, name-processing, ambiguous reference-rewrite, and cyclic reference-rewrite failures. A successful `TidyResult` can contain structured `TidyWarning` values for missing keys and duplicate entries.

## Polars failures

Value expressions turn row-local input, parse, missing-key, and render failures into null. Report expressions preserve parser, renderer, or formatter details in structs.

Invalid static options can raise during expression construction. Invalid dtypes, projection fields, styles, output-name collisions, and broadcasting lengths raise from Polars when the query executes.

Read [Troubleshooting](/troubleshooting) for the smallest recovery step for common failures.
