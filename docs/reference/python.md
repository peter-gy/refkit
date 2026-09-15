---
description: Look up every public refkit object, function, result, helper, and runtime metadata contract.
---

# Python API

The `refkit` package exports normalized bibliography objects, citation rendering objects, raw BibTeX views, formatting records, errors, helpers, and runtime metadata.

## Normalized bibliography

### `Library.read(path, *, recovery="error")`

Reads `.bib`, `.yaml`, or `.yml` and returns a `Library`. File extensions are matched case-insensitively. The recovery policy applies to `.bib` parsing.

### `Library.parse_bibtex(source, *, recovery="error")`

Parses BibTeX or BibLaTeX source in memory. `"error"` raises `ParseError` on parser diagnostics. `"report"` keeps recoverable entries and exposes structured records through `diagnostics`.

### `Library.parse_yaml(source)`

Parses a Hayagriva YAML bibliography in memory.

### `Library.from_records(records)`

Constructs a library from an iterable of `refkit.types.Entry` dictionaries. Each entry requires a nonempty `key` and canonical TitleCase `entry_type`. Omitted optional fields receive empty or null defaults. Duplicate top-level keys, unknown fields, invalid typed values, and exceeded resource bounds raise `ValueError`.

### `Library.from_json(source)` and `library.to_json()`

Read or write a RefKit record snapshot with `schema_version: 1` and a `records` array. The JSON field names are snake_case in both language bindings. Python and TypeScript can exchange these snapshots directly. Unknown schema versions raise `ValueError`.

### `Library`

| Member | Contract |
| --- | --- |
| `diagnostics` | Returns a new list of `refkit.types.Diagnostic` dictionaries. |
| `keys()` | Returns citation keys in library order. |
| `values()` / `to_records()` | Returns detached `Entry` dictionaries in library order. |
| `get(key)` | Returns one entry or `None`. |
| `get_many(keys)` | Returns entries in requested order. Missing keys raise `KeyError`. |
| `select(selector)` | Returns entries matched by a Hayagriva selector. |
| `project(fields=None, *, keys=None)` | Returns dictionaries for selected fields and keys. |
| `is_empty()` | Returns whether the library has no entries. |
| `len(library)` | Returns the entry count. |
| `bool(library)` | Returns whether the library has entries. |
| `key in library` | Tests key membership. |
| `library[key]` | Returns one entry. A missing key raises `KeyError`. |

`project()` defaults to `key`, `title`, `doi`, and `volume`. Read [Data Shapes](/reference/data-shapes) for projection fields.

### `Entry`

`Entry` is a typed dictionary containing structured text, creator names, dates, publication details, identifiers, nested parents, and namespaced extensions. Read fields with `entry["title"]` or `entry["authors"]`. Returned dictionaries contain every field, with null or empty defaults for missing values. Mutating a returned record leaves its library unchanged. Pass edited records to `Library.from_records` to construct another library.

Use `project()` for scalar title, date, DOI, and inherited volume values. [Bibliography records](/reference/data-shapes#bibliography-records) describes the complete structure.

## Bibliography validation

### `Library.validate()` and `BibDocument.validate()`

Return an inspect-only `ValidationReport`. `Library` uses the format-neutral record profile. `BibDocument` uses the BibLaTeX field profile and reports current-snapshot occurrence IDs and byte spans. Source parsing failures raise `ParseError`. Read [validation behavior](/concepts/parsing-and-recovery#validate-bibliography-data) and [report fields](/reference/data-shapes#validation-reports).

## Bibliography codecs

### `decode(source, *, format, loss="report", recovery="error")`

Returns a `DecodeReport` dictionary with `library`, `format`, and `issues`. Formats are `biblatex`, `hayagriva`, and `csl-json`. Recovery applies to BibLaTeX input. Malformed input and refused loss raise `ConversionError` with `issues` and parser `diagnostics`.

### `encode(library, *, format, loss="report")`

Returns an `EncodeReport` dictionary with `format`, `text`, and `issues`. `loss="error"` refuses reported loss and leaves the library unchanged.

### `convert(source, *, source_format, target_format, loss="report", recovery="error")`

Returns a `ConversionReport` with `source_format`, `target_format`, `text`, `issues`, and `diagnostics`. Read [Convert Bibliographies](/guides/convert-bibliographies) for format mappings and loss semantics.

## Styles and citations

### `Style.list()`

Returns bundled style metadata sorted by canonical `name`. Each `refkit.types.StyleMetadata` dictionary contains `name`, `aliases`, `title`, and `csl_id`. Pass a name or alias to `Style.load`.

### `Style.load(name)`

Loads a bundled independent CSL style. Lookup is case-insensitive. An unknown name raises `ValueError`.

### `Style.from_xml(xml, *, parent_xml=None)`

Prepares CSL XML. A dependent style requires `parent_xml` containing its independent parent, with a CSL identifier matching the child link. Invalid XML, mismatched parents, and invalid macro graphs raise `ValueError`. An independent style rejects supplied parent XML. `id` is `"xml"`.

### `Style.from_path(path)`

Reads strict UTF-8 CSL XML and prepares an independent style. `id` is the path string.

`Style.title` and `Style.csl_id` identify the requested style, including a dependent child. `Style.id` identifies how RefKit loaded the style and is not guaranteed to equal the CSL `<id>` element. A child's default locale overrides its parent's default. An explicit document locale overrides both.

### `Locale.load(code)`

Validates a bundled CSL locale code. `code` returns the validated code.

### `Cite(key, *, locator=None, label=None, purpose="normal")`

Creates one cite item. A locator with no label uses the `page` label. A label with no locator has no rendering effect. Invalid labels raise `ValueError` when rendering runs.

`purpose` accepts `normal`, `author`, `year`, `full`, or `prose` and is returned by the property of the same name. An unknown purpose raises `ValueError` during construction. [Citation purposes](/guides/render-citations#choose-a-citation-purpose) describes their output.

### `CitationGroup(items)`

Creates one group from an iterable of citation-key strings and `Cite` objects. A plain string is not accepted as the iterable. An empty group raises `ValueError`.

`items` returns a new list of normalized `Cite` objects. `len(group)` returns its item count.

### `Citation(id, citation, *, note_number=None)`

Creates a named citation from a key string, `Cite`, or `CitationGroup`. `id` names the result inside one render call. `group` returns the normalized `CitationGroup`. `note_number` identifies the document note containing this occurrence and is returned by the property of the same name.

## Rendering

### `Document(library, style, *, locale=None)`

Stores a prepared `Library`, `Style`, and optional locale code. `locale` accepts a string, `Locale`, or `None`. Use `Locale.load` when construction-time validation is required because raw strings are passed to the renderer.

### `Document.render(citations)`

Renders an iterable of uniquely named `Citation` objects with fresh citation state. Returns `RenderedDocument`. Missing citation keys raise `MissingReferenceError`.

### `Document.cited_bibliography(citations)`

Renders the bibliography for one ordered iterable of uniquely named `Citation` objects. Returns `Rendered`. Duplicate IDs or locator labels raise `ValueError`. Missing keys raise `MissingReferenceError`. Renderer failures raise `RefkitError`.

### `Document.full_bibliography()`

Renders every entry in the library with fresh citation state. Returns `Rendered`.

### `RenderedDocument`

| Member | Contract |
| --- | --- |
| `citation_order` | New list of result IDs in input order. |
| `citations` | New dictionary from ID to `Rendered`. |
| `bibliography` | Cited bibliography as `Rendered`. |
| `rendered[id]` | Named citation. A missing ID raises `KeyError`. |

### `Rendered`

`text` returns plain text. `html` returns rendered HTML. `tree` returns fresh JSON-shaped Python data. `layout` returns a `BibliographyLayout` dictionary for bibliography output and `None` for a citation. Read [Data Shapes](/reference/data-shapes) for the tree and layout protocols.

## Raw BibTeX

### `BibDocument.read(path)` and `BibDocument.parse(source)`

Create an immutable raw BibTeX snapshot from a file or string. Malformed blocks remain available through `failed_blocks`. Snapshots and their handles can be read across Python threads.

| Member | Contract |
| --- | --- |
| `entries` | Snapshot-bound `BibEntryMap` view. |
| `diagnostics` | Source decode diagnostics as `Diagnostic` dictionaries. |
| `comments` | New source-order list of comment strings. |
| `preamble` | Preamble values joined with ` # `. |
| `strings` | New dictionary of string definitions sorted by key. |
| `failed_blocks` | New list of malformed block records. |
| `blocks` | New list of every source-order block record. |
| `to_bibtex()` | Serializes the current in-memory state. |
| `tidy(*, options=None)` | Strictly formats the current state into `TidyResult`. |
| `write(path)` | Writes the current state as UTF-8. |

### `BibDocument.apply_patch(patch)`

Accepts an iterable of `refkit.types.BibEdit` records and returns `BibPatchResult` containing a new `document`, byte `changes`, occurrence mappings in `entries`, and `warnings`. The input snapshot and its handles remain unchanged. `PatchError` carries `code` and nullable input `operation` index. Read [atomic patch behavior](/guides/edit-bibtex#apply-structural-changes-atomically) and [report fields](/reference/data-shapes#patch-reports).

### `BibDocument.find_duplicates(*, rules=None)`

Returns `DuplicateReport` with source-relative candidate groups, rule signatures and conflicting values. The default rules are `doi`, `key`, `abstract`, and `citation`. An empty iterable selects no rules. Matching is inspect-only.

### `BibDocument.plan_merge(entries, *, retain, fields=None, entry_type=None)`

Select at least two distinct input entry IDs and a retained member. Returns `MergePlan` with a nullable `patch` and unresolved `conflicts`. Field choices take a source occurrence or drop a field. Entry-type conflicts require `entry_type`. Invalid selections, choices, and unsafe reference transformations raise `MergeError`. [Review duplicates](/guides/review-duplicates) before applying the returned patch to the same snapshot.

### `BibDocument.resolve()`

Returns `list[refkit.types.ResolvedBibEntry]` in source order. Each record has
`key`, `entry_type`, and a `fields` dictionary containing every source field
with string macros and concatenations expanded. Results are detached from the
document and describe its snapshot. Invalid or ambiguous input raises
`ParseError` with diagnostics. See [field resolution](/guides/edit-bibtex#resolve-fields-for-inspection).

### `BibEntryMap` and `BibFieldMap`

Both lengths count source occurrences, including duplicates. `unique_keys()` counts names through its returned list.

Both lookup views provide `unique_keys()`, `occurrence_keys()`, `occurrences()`, `get_all(key)`, `get_unique(key)`, `is_empty()`, length, truthiness, membership, and item lookup. They are focused views and do not implement the full Python `Mapping` interface.

Entry keys are case-sensitive. Field lookup is case-insensitive. A missing direct lookup raises `KeyError`. An ambiguous direct lookup raises `RefkitError` and should be replaced with `get_all`.

### `BibEntry`

`id` is the snapshot-relative occurrence index. `key` returns the raw citation key, `kind` returns the raw entry type spelling, `fields` returns a snapshot-bound `BibFieldMap`, and `span` returns a half-open byte span.

### `BibField`

`id`, `entry_id`, `name`, `value`, and `span` are read-only properties of one raw field occurrence in its original snapshot. Pass its IDs to `apply_patch` to create an edited snapshot.

## Formatting helpers

### `tidy_bibtex(source, *, options=None)`

Formats a BibTeX string and returns `TidyResult`.

### `tidy_file(path, *, output=None, options=None)`

Reads and formats a BibTeX file. When `output` is set, writes the formatted result to that path.

### `TidyResult` and `TidyWarning`

`TidyResult.bibtex` is formatted source. `count` is the input entry count, including entries merged from output. `warnings` is a list of `TidyWarning` records. `renames` is a source-order list of `TidyRename` dictionaries with `entry_id`, `old_key`, and `new_key`. Use it to update citation keys outside the bibliography.

A warning exposes `code`, optional duplicate `rule`, and `message`. Codes are `missing_key` and `duplicate_entry`.

Read [Tidy Options](/reference/tidy-options) for `TidyOptions`.

## Path Helpers

### `cite(source, citation, *, style="apa", locale="en-US")`

Reads a bibliography path and renders one citation. `citation` accepts a key string, `Cite`, or `CitationGroup`. `style` accepts a bundled name or prepared `Style`.

### `full_bibliography(source, *, style="apa", locale="en-US")`

Reads a bibliography path and renders every normalized entry.

## Typed records

Import dictionary and tree types from `refkit.types`:

```python
from refkit.types import Diagnostic, ProjectionRow, RenderedTree, TidyRename
```

These runtime `TypedDict` definitions describe records returned by RefKit. They can be inspected by type and schema tools. [Data Shapes](/reference/data-shapes) defines their keys, values, and nullability.

## Runtime metadata

`__version__` is the installed distribution version. `build_info` identifies the RefKit version, operating system, and architecture. `build_mode` is `"debug"` or `"release"`.

Import verifies that package metadata and the native extension have the same version. A mismatch raises `SystemError`. Reinstall one complete RefKit release to restore the pair.
