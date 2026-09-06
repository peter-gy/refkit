# Core Semantics

`crates/refkit-core` owns the portable behavior shared by every adapter. This page records the subsystem invariants that are too detailed for the architecture map.

## Decode And Normalize

`decode_bibliography(bytes)` decodes the complete input as UTF-8 when valid. Any UTF-8 failure selects the Windows-1252-compatible table for the complete byte sequence. Undefined control bytes become Unicode replacement characters. The returned `TextEncoding` records the choice.

The normalized parser has two recovery policies:

- `Error` uses the exact parser and returns all parse errors joined by newlines.
- `Report` sanitizes recoverable source and returns ordered diagnostics beside entries.

Report recovery keeps the first duplicate key, removes later duplicate blocks, literalizes unresolved string expressions where possible, drops invalid typed fields, and can drop an entry whose fields cannot identify a usable type. Syntax recovery is capped at 16 block-removal passes. Per-entry typed-field recovery is capped at 64 passes.

An empty source can create an empty `Library`. A non-empty source that recovers to zero entries with diagnostics fails. Keep this distinction in every adapter's null or exception mapping.

## Normalized Records

`Library` caches owned keys and `EntryRecord` projections lazily. An `EntryRecord` contains its key, normalized entry type, common scalar fields, and recursive parents. Volume falls back to the first parent volume.

`EntryField` accepts `entry_type` and `type` as aliases. Use `entry_type` as the canonical record name and keep `type` as a boundary alias while it remains public.

Hayagriva selectors parse at runtime and match normalized entry structure. Adapters return top-level matched records and do not expose selector bindings.

## Raw Syntax And Occurrences

`RawDocument` preserves whitespace, comments, preambles, string definitions, entries, failed blocks, and other text. Every block carries a half-open UTF-8 byte span.

Raw entry keys are case-sensitive. Raw field lookup is case-insensitive. Duplicate entry and field occurrences retain source order and document-local positional IDs. An ID from another `RawDocument` has no meaning in the current instance.

Raw values have five modes: bare, braced, quoted, concatenated expression, and missing. An expression preserves each atom and its delimiter mode.

The parser can recognize a complete `@...` block that begins later on a percent-comment line. Normalized recovery suppresses that embedded block. Raw formatting can render it as a live entry. Preserve focused tests around this boundary until the parser contract changes.

## Raw Edit Lifecycle

`set_field_value` is the structural mutation boundary. It edits an existing field occurrence.

1. Resolve the document-local field ID.
2. Validate the replacement against the original value mode.
3. Store a replacement for the field-value span.
4. Serialize unchanged entries from their original bytes.
5. Apply changed value-span patches inside changed entry slices.
6. Preserve every unrelated block.

Bare values remain bare while safe. Complex bare replacements become braced. Editing a concatenated expression replaces the complete expression with one braced value. Invalid newlines, percent delimiters, trailing escapes, brace balance, or quote boundaries fail before mutation.

## Tidy Pipeline

`tidy_bibtex` performs these stages:

1. Normalize line endings to LF.
2. Parse the raw syntax and reject the first failed block.
3. Collect missing-key warnings.
4. Plan generated keys.
5. Plan duplicate warnings and merges.
6. Sort and render blocks and fields.
7. Apply value transforms.
8. End output with one newline.

`TidyResult.count` is the parsed occurrence count before duplicate merges. Concatenated expressions preserve their atoms and bypass the scalar value transforms.

Duplicate DOI and abstract matching ignores case and non-alphanumeric differences. Abstract matching uses the first 100 normalized characters. Citation matching combines the first author surname, title, and number. When duplicate detection or merging is enabled, duplicate-key matches are reported and are not merged implicitly.

The public [tidy option reference](../docs/reference/tidy-options.md) owns the 27 user-facing arguments and defaults. Core tests own option interactions, key-template grammar, and output invariants.

## Render Lifecycle

Core `Document` stores immutable library and style handles plus an optional locale code. Each render, cited-bibliography, or full-bibliography call creates a fresh `BibliographyDriver`.

Within one render call, citation order can affect numbering, position-sensitive formatting, disambiguation, subsequent-name rules, and the cited bibliography. A missing reference or invalid locator label fails the complete operation before output is returned.

Lower-level single, each, and group render functions support stateless adapters. Style analysis switches repeated citation rendering to a shared driver when citation numbers, citation position, subsequent-name behavior, or ambiguous standalone text requires context.

`render_library_bibliography(all=false)` registers no entries and returns empty output. It is not a cited-subset operation. Use `Document::cited_bibliography` for that contract.

## HTML And Render Trees

The HTML renderer escapes text and attributes. It permits `http`, `https`, and `mailto` links. Other schemes become formatted text.

Typed render nodes preserve text formatting, element display, metadata, markup, links, transparent fragments, and bibliography entry identity. `Transparent` nodes produce no HTML. `Markup` remains typed tree data and is escaped in HTML output.

A bibliography record stores `first_field` separately and also prepends that node to `children`. Consumers should traverse `children` once. Preserve the current shape in adapter conversion and exact-shape tests until the protocol changes.
