# Core Semantics

`crates/refkit-core` owns the portable behavior shared by every adapter. This page records the subsystem invariants that are too detailed for the architecture map.

## Decode And Normalize

`decode_bibliography(bytes)` decodes the complete input as UTF-8 when valid. Any UTF-8 failure selects the Windows-1252-compatible table for the complete byte sequence. Undefined control bytes become Unicode replacement characters. The returned `TextEncoding` records the choice.

The normalized parser has two recovery policies:

- `Error` uses the exact parser and returns typed diagnostics in `ParseFailure`.
- `Report` repairs affected blocks or fields and returns ordered diagnostics beside entries.

Report recovery keeps the first duplicate key, drops later duplicate blocks, repairs unresolved string atoms where possible, drops invalid typed fields, and can drop an entry whose fields cannot identify a usable type. It preserves valid macro definitions and unaffected normalized values. Independent repairs are batched. Diagnostics retain original UTF-8 spans through same-length source masks and edits to parsed values.

An empty source can create an empty `Library`. A non-empty source that recovers to zero entries with diagnostics fails. Keep this distinction in every adapter's null or exception mapping.

Cycle and expansion checks run before recursive upstream normalization. Source and expanded data are bounded at 16 MiB, value and dependency nesting at 64 levels, and dependency traversal at 100,000 steps. Cascading recovery is bounded by 100,000 revisited raw atoms and 128 MiB of cumulative source reparsing. Exceeding a budget returns a resource-limit failure. Report recovery can remove rejected components while retaining independent valid entries.

## Normalized Records

`Library` owns complete immutable `EntryRecord` values and an index. It derives a private Hayagriva library from those records for rendering and selectors. Structured construction validates the complete record set before returning. Scalar projection flattens text and dates and falls back to the first parent volume, while complete records preserve own values, creator metadata, date ranges, protected/math text, and source-specific extensions.

`EntryField` uses `entry_type` as the canonical type field. The JavaScript adapter spells it `entryType`. Version-one record snapshots use snake_case throughout. Serde and serde_json are pure implementation libraries for the owned record schema and bibliography interchange.

Hayagriva selectors parse at runtime and match normalized entry structure. Adapters return top-level matched records and do not expose selector bindings.

## Interchange

The finite codec set accepts BibLaTeX, Hayagriva YAML, and CSL-JSON. Decode reports own a library and conversion issues. Encode reports own target text and issues. Conversion reports combine source/target identity, issues, and parser diagnostics. Loss refusal is atomic and leaves input libraries unchanged.

Encoders decode their generated source and compare structured record values and nonredundant source extensions. Source annotations beginning with `@` describe provenance and are excluded from that comparison. Explicit issue rules cover known numeric, date, and source-type approximations. Keep parser diagnostics separate from conversion issues, including during recovery. The CSL codec uses owned records and never sends unvalidated date ranges to the engine's CSL-JSON rendering adapter.

## Bibliography Validation

`Library::validate` inspects complete records. `RawDocument::validate` parses the current rendered snapshot under the shared dependency and resource guards, then applies the pinned BibLaTeX field profile to effective inherited fields. Parser failures stay `ParseFailure` values. Validation reports have their own codes, severity, occurrence targets and available UTF-8 spans.

Both operations leave data unchanged. Identifier syntax and checksum checks are deterministic and perform no resolution. Matching top-level identifiers produce review groups. Repeated embedded containers are excluded from those groups. Partial date ranges are reported as reversed only when their calendar bounds establish that ordering.

## Raw Syntax And Occurrences

`RawDocument` preserves whitespace, comments, preambles, string definitions, entries, failed blocks, and other text. Every block carries a half-open UTF-8 byte span.

Raw entry keys are case-sensitive. Raw field lookup is case-insensitive. Duplicate entry and field occurrences retain source order and document-local positional IDs. An ID from another `RawDocument` has no meaning in the current instance.

Raw values have five modes: bare, braced, quoted, concatenated expression, and missing. An expression preserves each atom and its delimiter mode.

The parser can recognize a complete `@...` block that begins later on a percent-comment line. Normalized recovery suppresses that embedded block. Raw formatting can render it as a live entry. Preserve focused tests around this boundary until the parser contract changes.

## Raw Edit Lifecycle

`RawDocument::apply_patch` accepts snapshot-relative `BibEdit` operations and returns a new document, byte changes, occurrence mappings, and warnings. Raw snapshots and their handles are immutable.

1. Validate occurrence targets, authored values and resource bounds.
2. Reject conflicting operations on the same field, key, type or removed entry.
3. Plan byte replacements and unambiguous reference rewrites using shared reference-key encoding.
4. Apply nonoverlapping replacements while copying every unrelated byte.
5. Parse the result and verify retained entry and field boundaries.
6. Return old-to-new occurrence mappings and spans. Original handles retain the input snapshot.

Bare values remain bare while safe. Complex bare replacements become braced. Editing a concatenated expression replaces the complete expression with one braced value. New fields and entries use braced values. Invalid newlines, percent delimiters, trailing escapes, brace balance, or quote boundaries fail atomically. Explicit reference edits supply final values and are excluded from automatic rewrites.

## Tidy Pipeline

Duplicate review and tidy share signature generation for DOI, key, abstract and citation rules. Review reports expose the signature and its members, including transitive candidate groups. Expression-level field conflicts are separate from matching evidence. Distinct valid canonical identifiers receive identifier-conflict classification.

Merge planning keeps source-order membership and explicit retained-entry and field choices. Conflicts prevent producing a patch. Accepted plans copy complete value expressions, rewrite unambiguous references, reject cycles, and validate the generated patch through `RawDocument::apply_patch`. Planning leaves the source unchanged and introduces no second serializer. Patch expression mode validates one complete raw value, preserving macros, concatenations, percent-containing URLs and multiline braced values.

`tidy_bibtex` performs these stages:

1. Normalize line endings to LF.
2. Parse the raw syntax and reject the first failed block.
3. Collect missing-key warnings.
4. Plan duplicate warnings and merges.
5. Assign globally unique keys to retained entries.
6. Rewrite bibliography references and record source-occurrence renames.
7. Sort using emitted keys and render blocks and fields.
8. Apply value transforms and end output with one newline.

`TidyResult.count` is the parsed occurrence count before duplicate merges. `renames` maps source occurrence IDs and old keys to final keys, including merged occurrences. Ambiguous source-key references fail the transformation before output. Concatenated expressions preserve their atoms and bypass the scalar value transforms.

Duplicate DOI and abstract matching ignores case and non-alphanumeric differences. Abstract matching uses the first 100 normalized characters. Citation matching combines the first author surname, title, and number. When duplicate detection or merging is enabled, duplicate-key matches are reported and are not merged implicitly.

The public [tidy option reference](../docs/reference/tidy-options.md) owns the 27 user-facing arguments and defaults. Core tests own option interactions, key-template grammar, and output invariants.

## Render Lifecycle

Custom style preparation bounds XML source at 2 MiB, 100,000 nodes, 64 nested elements, and 256 attributes per element, including namespace declarations, before deserialization. Macro validation bounds expanded rendering work at 100,000 elements and combined rendering-element/macro depth at 64 levels. Mutually exclusive conditional branches contribute their maximum expansion work.

Core `Document` stores immutable library and style handles plus an optional locale code. Each render, cited-bibliography, or full-bibliography call creates a fresh `BibliographyDriver`.

Within one render call, citation order can affect numbering, position-sensitive formatting, disambiguation, subsequent-name rules, and the cited bibliography. A missing reference or invalid locator label fails the complete operation before output is returned.

`CitationRequest` owns cite items and an optional note number. Single, ordered-list, and grouped render operations use the same driver semantics. A grouped request shares numbering, sorting, and disambiguation within the group. Independent Polars rows keep independent processor state.

Each `Cite` has a `CitePurpose`: `Normal`, `Author`, `Year`, `Full`, or `Prose`. The core maps purposes to the renderer before processing the ordered requests. Text, HTML, and trees derive from that same render. The style catalog returns owned metadata sorted by canonical load name, with aliases and CSL identifiers.

Style preparation accepts a dependent source with an explicitly supplied independent parent source. Both sources pass XML bounds before deserialization. The parent identifier must exactly match the child link, and the parent's macros pass the rendering-work limits. `PreparedStyle` retains child title and CSL identity alongside the prepared parent engine rules. Locale precedence is explicit document locale, child default, parent default, then engine fallback.

`render_library_bibliography` renders every library entry. `Document::cited_bibliography` renders the entries named by its ordered requests.

## HTML And Render Trees

The HTML renderer escapes text and attributes. It permits `http`, `https`, and `mailto` links. Other schemes become formatted text.

Typed render nodes preserve text formatting, element display, metadata, markup, links, transparent fragments, and bibliography entry identity. `Transparent` nodes produce no HTML. `Markup` remains typed tree data and is escaped in HTML output.

A bibliography record separates its `label` from `content`. `BibliographyLayout` retains hanging indent, second-field alignment, line spacing, and entry spacing. Tagged element metadata retains entry and name identity. Finite formatting and display values are RefKit-owned enums. `RenderedRecord` caches the owned tree behind the portable API.
