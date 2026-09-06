# RefKit Contracts

## State owners

| Object | Owns |
| --- | --- |
| `Library` | Normalized entries and parser diagnostics. |
| `BibDocument` | Source-order raw BibTeX, occurrence identity, and preserving writeback. |
| `Style` | A prepared Citation Style Language style. |
| `Document` | Prepared library, style, and locale inputs. |
| One render call | Fresh ordered citation processor state and its cited bibliography. |
| `Rendered` | Text, HTML, and a structured render tree for one result. |

## Input boundaries

- `Library.read` infers BibTeX, BibLaTeX, or Hayagriva YAML from a filesystem extension.
- `Library.parse_bibtex` accepts BibTeX or BibLaTeX text.
- `Library.parse_yaml` accepts Hayagriva bibliography YAML.
- `BibDocument` accepts raw BibTeX and preserves blocks that normalized parsing does not expose.
- `tidy_bibtex` accepts raw BibTeX text and rejects the first malformed block with `TidySyntaxError`.

## Duplicate boundaries

Normalized `Library` keys are unique. Report recovery retains the first recoverable entry for a duplicate key and records a diagnostic.

Raw `BibDocument` entries and fields preserve duplicate occurrences. Direct lookup requires one match. Use `get_all` to select a source-order occurrence before editing.

## Render boundaries

A `Cite` identifies one bibliography key with an optional locator and label. A `CitationGroup` combines cite items into one rendered citation. A `Citation` gives the rendered occurrence an ID used to retrieve it from `RenderedDocument`.

Citation order can affect numbering, disambiguation, position-sensitive formatting, subsequent-name rules, and the cited bibliography. Separate render calls have independent processor state.

## Error boundaries

| Error | Use |
| --- | --- |
| `RefkitError` | Filesystem, parser, raw ambiguity, and renderer failures. |
| `MissingReferenceError` | A render request names a key absent from the `Library`. |
| `TidyError` | Key generation or another formatting operation fails. |
| `TidySyntaxError` | Raw BibTeX contains a malformed block. Inspect its line, column, byte, character, and message fields. |
| `TypeError`, `ValueError`, `KeyError` | Python shape, option, label, lookup, and identifier failures. |

## Output discipline for notebook agents

- Show a bounded projection before printing complete source.
- Prefer `Rendered.text` for inspection and `Rendered.html` for an HTML consumer.
- Traverse `Rendered.tree` when structured formatting or link metadata is required.
- Report diagnostics and tidy warnings beside the affected input.
- Preview preserving writes and generated keys before committing filesystem changes.
