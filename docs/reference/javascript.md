---
description: Look up refkit-js initialization, bibliography objects, rendering, raw edits, formatting, and Node file helpers.
---

# TypeScript API

`refkit-js` exports bibliography classes and typed result records for Node.js
and browsers. [Get Started](/get-started) covers installation and the first rendering.
[Run in a Browser](/guides/browser) covers browser assets and initialization. The package includes
[TypeScript](https://www.typescriptlang.org/) declarations for editor completion
and compile-time checking of every exported signature and record.

## Imports and initialization

| Import | Contract |
| --- | --- |
| `refkit-js` | Shared API and Node helpers under Node.js, with WebAssembly initialized during import. Browser resolution requires `await init()`. |
| `refkit-js/browser` | Shared API with bindings and WebAssembly deferred until `init()`. |
| `refkit-js/node` | Shared API and asynchronous filesystem helpers for Node.js. |
| `refkit-js/refkit.wasm` | Packaged WebAssembly asset for bundlers and deployment tooling. |

### `init(input?)`

Dynamically loads the generated bindings and packaged WebAssembly module and returns
a `Promise<void>`. Importing the browser entry leaves this loading operation deferred. With no argument,
the browser loader resolves the packaged `.wasm` file relative to its module.
Pass a URL for a separately served asset. `InitOptions` also accepts a request,
response, byte buffer, compiled `WebAssembly.Module`, a promise for those inputs,
or a `{ module_or_path: input }` object. Concurrent calls share one promise and use
the first call's input. A failed WebAssembly download or compilation rejects the promise and
allows a subsequent call to retry.
Calls after successful initialization resolve immediately and retain the loaded module.
Await initialization before parsing, rendering, formatting, loading styles or locales,
or calling `getBuildInfo()`. Citation descriptions such as `Cite` and `Citation`
can be constructed before initialization. Read the [browser deployment requirements](/guides/browser#initialize-the-module)
for asset delivery and content security policy.

## Normalized bibliography

### `Library.parseBibtex(source, { recovery = "error" } = {})`

Parses [BibTeX](https://ctan.org/pkg/bibtex) or
[BibLaTeX](https://ctan.org/pkg/biblatex) reference text and returns a `Library`. `"error"` throws
`ParseError` on parser diagnostics. `"report"` retains recoverable entries and
exposes diagnostics. A source that cannot produce a recovered library still
throws `ParseError`.

### `Library.parseYaml(source)`

Parses a [Hayagriva YAML](https://github.com/typst/hayagriva) bibliography,
a structured reference format, and returns a `Library`.

### `Library.fromRecords(records)`

Constructs a library from an iterable of `Entry` records. Each requires a nonempty `key` and canonical TitleCase `entryType`. Omitted optional fields receive empty or null defaults. Duplicate top-level keys, unknown fields, invalid typed values, and exceeded resource bounds throw `RangeError`.

### `Library.fromJson(source)` and `library.toJson()`

Read or write a RefKit record snapshot with `schema_version: 1` and a `records` array. Snapshot field names are snake_case in both language bindings. Python and TypeScript can exchange these snapshots directly. Unknown schema versions throw `RangeError`.

### `Library`

| Member | Contract |
| --- | --- |
| `diagnostics` | Structured parser diagnostics. |
| `size` | Number of normalized entries. |
| `keys()` / `values()` | Keys or `Entry` records in library order. |
| `toRecords()` | Detached complete records in library order. |
| `get(key)` | `Entry` or `null`. |
| `getMany(keys)` | Entries in requested order, including repeated keys. Throws for a missing key. |
| `has(key)` / `isEmpty()` | Membership or emptiness. |
| `select(selector)` | Entries matched by a [selector](/reference/selectors). |
| `project(fields?, { keys } = {})` | Records containing requested fields, optionally restricted to ordered keys. |

`project()` defaults to `key`, `title`, `doi`, and `volume`. Supported fields
are `key`, `entryType`, `title`, `date`, `doi`, and `volume`. Missing selected keys throw. A missing field value is `null`.

`Entry` records contain structured text, creator names, dates, publication details, identifiers, nested parents, and namespaced extensions. Returned records contain every field, with null or empty defaults for missing values. Mutating a returned record leaves its library unchanged. A `Library` is iterable over its records. Use `project()` for scalar title, date, DOI, and inherited volume values. [Bibliography records](/reference/data-shapes#bibliography-records) describes the complete structure.

## Bibliography validation

### `Library.validate()` and `BibDocument.validate()`

Return an inspect-only `ValidationReport`. `Library` uses the format-neutral record profile. `BibDocument` uses the BibLaTeX field profile and reports current-snapshot occurrence IDs and byte spans. Source parsing failures throw `ParseError`. Read [validation behavior](/concepts/parsing-and-recovery#validate-bibliography-data) and [report fields](/reference/data-shapes#validation-reports).

## Bibliography codecs

### `decode(source, { format, loss = "report", recovery = "error" })`

Returns `DecodeReport` with `library`, `format`, and `issues`. Formats are `biblatex`, `hayagriva`, and `csl-json`. Recovery applies to BibLaTeX input. Malformed input and refused loss throw `ConversionError` with `issues` and parser `diagnostics`.

### `encode(library, { format, loss = "report" })`

Returns `EncodeReport` with `format`, `text`, and `issues`. `loss: "error"` refuses reported loss and leaves the library unchanged.

### `convert(source, { sourceFormat, targetFormat, loss = "report", recovery = "error" })`

Returns `ConversionReport` with `sourceFormat`, `targetFormat`, `text`, `issues`, and `diagnostics`. Read [Convert Bibliographies](/guides/convert-bibliographies) for format mappings and loss semantics.

## Styles and citation inputs

### `Style.list()`

Returns `StyleMetadata[]` sorted by canonical `name`. Each record contains `name`, `aliases`, `title`, and `cslId`. Pass a name or alias to `Style.load`.

### `Style.load(name)` and `Style.fromXml(xml, { parentXml = null } = {})`

Return a prepared [Citation Style Language](https://citationstyles.org/)
style, which controls citation and bibliography formatting. Bundled lookup is
case-insensitive. A dependent style requires `parentXml` containing its independent parent, with a CSL identifier matching the child link. Invalid XML, mismatched parents, invalid macro graphs, and unknown bundled names throw. An independent style rejects supplied parent XML.

`title` and `cslId` identify the requested style, including a dependent child. `id` is the requested bundled name or `"xml"` for an XML input. A child's default locale overrides its parent's default. An explicit document locale overrides both.

### `Locale.load(code)`

Validates a bundled locale and returns a `Locale` with its `code` property.

### `new Cite(key, { locator = null, label = null, purpose = "normal" } = {})`

Creates a citation item. A locator with an omitted label uses `page`. A label
has a rendering effect when a locator is supplied. Invalid labels throw at
render time.

`purpose` accepts `normal`, `author`, `year`, `full`, or `prose` and is returned by the property of the same name. An unknown purpose throws `RangeError` during construction. [Citation purposes](/guides/render-citations#choose-a-citation-purpose) describes their output.

### `new CitationGroup(items)`

Creates a group from an iterable of key strings and `Cite` objects. Empty
groups and a bare string passed as `items` throw. `items` exposes the normalized
`Cite` array, and `size` gives its length.

### `new Citation(id, citation, { noteNumber = null } = {})`

Creates a named occurrence from a key string, `Cite`, or `CitationGroup`.
`id` identifies its result within one render call. `group` exposes the
normalized citation group. `noteNumber`, when supplied, is an integer from
1 through 4294967295 identifying the document note.

## Rendering

### `new Document(library, style, { locale = null } = {})`

Prepares a document with a library, style, and optional locale string or
`Locale`. Use `Locale.load(code)` for explicit locale validation. Each render
operation creates fresh citation state.

| Method | Result |
| --- | --- |
| `render(citations)` | `RenderedDocument` containing named citations and the cited bibliography. |
| `citedBibliography(citations)` | `Rendered` bibliography restricted to the ordered citation inputs. |
| `fullBibliography()` | `Rendered` bibliography containing every library entry. |

`citations` is an iterable of `Citation` objects with unique IDs. Missing
bibliography keys throw `MissingReferenceError`. Duplicate IDs and invalid
note numbers or locator labels throw.

### `RenderedDocument`

`citationOrder` is the input-order array of IDs. `citations` maps IDs to
`Rendered` records. `get(id)` returns a citation and throws for a missing ID.
`bibliography` is the cited bibliography.

### `Rendered`

`text` is plain text, `html` is rendered HTML, and `tree` is a typed node array.
`layout` contains bibliography spacing and alignment, or `null` for citation
output. Returned records are JavaScript data that can outlive their document.

The tree retains text formatting, links, source entry metadata, and separate
bibliography labels and content. It uses the same discriminants and enum
values as [Data shapes](/reference/data-shapes), with camelCase property names:
`itemIndex`, `citeIdx`, `fontStyle`, `fontVariant`, `fontWeight`,
`textDecoration`, `verticalAlign`, `hangingIndent`, `secondFieldAlign`,
`lineSpacing`, and `entrySpacing`. The exported `RenderedTree`, `RenderedNode`,
`RenderedMeta`, and `BibliographyLayout` types define each variant.

### `cite(library, citation, { style = "apa", locale = "en-US" } = {})`

Renders one key string, `Cite`, or `CitationGroup` and returns `Rendered`.
`style` accepts a bundled name or prepared `Style`.

### `fullBibliography(library, { style = "apa", locale = "en-US" } = {})`

Renders every entry and returns `Rendered`. Both convenience helpers accept
a `Library`. Node callers read paths with `readLibrary()` first.

## Raw BibTeX

### `BibDocument.parse(source)`

Creates an immutable raw bibliography snapshot with entry and field views. Malformed blocks
remain available through `failedBlocks`.

| Member | Contract |
| --- | --- |
| `entries` | Snapshot-bound `BibEntryMap`. |
| `diagnostics` | Source decoding diagnostics. |
| `comments` | Source-order comment strings. |
| `preamble` | Preamble values joined with ` # `. |
| `strings` | String definitions keyed by name. |
| `failedBlocks` / `blocks` | Failed blocks or every source-order block. |
| `toBibtex()` | Returns the snapshot source. |
| `resolve()` | Returns detached `ResolvedBibEntry` records with expanded source fields. |
| `tidy({ options } = {})` | Formats the snapshot into a `TidyResult`. |

`BibEntry` exposes `id`, `key`, `kind`, `fields`, and `span`. `BibField` exposes
`id`, `entryId`, `name`, `value`, and `span`. These properties describe the
handle's original snapshot and are read-only. `tidy()` returns a formatted
result and preserves the current raw document.

`resolve()` returns source-ordered records with `key`, `entryType`, and `fields`.
It reads the snapshot and raises `ParseError` for invalid or ambiguous
input. See [field resolution](/guides/edit-bibtex#resolve-fields-for-inspection)
for macro handling and diagnostics.

### `BibDocument.applyPatch(patch)`

Accepts an iterable of `BibEdit` records and returns `BibPatchResult` containing a new `document`, byte `changes`, occurrence mappings in `entries`, and `warnings`. The input snapshot and its handles remain unchanged. `PatchError` carries `code` and nullable input `operation` index. Read [atomic patch behavior](/guides/edit-bibtex#apply-structural-changes-atomically) and [report fields](/reference/data-shapes#patch-reports).

### `BibDocument.findDuplicates({ rules = null } = {})`

Returns `DuplicateReport` with source-relative candidate groups, rule signatures and conflicting values. The default rules are `doi`, `key`, `abstract`, and `citation`. An empty iterable selects no rules. Matching is inspect-only.

### `BibDocument.planMerge({ entries, retain, fields = null, entryType = null })`

Select at least two distinct input entry IDs and a retained member. Returns `MergePlan` with a nullable `patch` and unresolved `conflicts`. Field choices take a source occurrence or drop a field. Entry-type conflicts require `entryType`. Invalid selections, choices, and unsafe reference transformations throw `MergeError`. [Review duplicates](/guides/review-duplicates) before applying the returned patch to the same snapshot.

### `BibEntryMap` and `BibFieldMap`

| Member | Contract |
| --- | --- |
| `size` | Number of source occurrences, including duplicates. |
| `uniqueKeys()` | Distinct keys. |
| `occurrenceKeys()` / `occurrences()` | Keys or snapshot-bound views in source order. |
| `getAll(key)` | Every occurrence of a key. |
| `getUnique(key)` | One occurrence or `null`. Throws `RefkitError` for ambiguity. |
| `has(key)` / `isEmpty()` | Membership or emptiness. |

Entry keys are case-sensitive. Field lookup is case-insensitive. Both maps
are iterable over their source occurrences.

`span` is a half-open `[start, end]` pair of UTF-8 byte offsets into the original
source. JavaScript string slicing uses UTF-16 indexes, so convert offsets
before slicing non-ASCII text. Raw block records use the discriminants listed
in [Data shapes](/reference/data-shapes#raw-blocks-and-spans).

## Formatting

### `tidyBibtex(source, { options } = {})`

Formats a source string and returns `TidyResult`. `options` accepts a plain
object matching the exported `TidyOptions` interface.

`TidyResult` contains `bibtex`, `count`, `warnings`, and `renames`. `count` is
the input entry count, including merged entries. A rename contains `entryId`,
`oldKey`, and `newKey`. Warnings contain `code`, `rule`, and `message`. Their
codes are `missing_key` and `duplicate_entry`.

### `TidyOptions`

The shared [Tidy options](/reference/tidy-options) reference lists every
Python and TypeScript name, default, boolean shorthand, and key-template rule.

## Node filesystem helpers

Import these asynchronous functions from `refkit-js/node`. Paths accept a
string or file URL.

| Function | Contract |
| --- | --- |
| `readLibrary(path, { recovery = "error" } = {})` | Reads `.bib`, `.yaml`, or `.yml` into a `Library`. Extension matching is case-insensitive. |
| `readBibDocument(path)` | Reads a raw `BibDocument`. |
| `readStyle(path)` | Reads strict UTF-8 custom style XML into a `Style` whose `id` is the path string. |
| `tidyFile(path, { output, options } = {})` | Returns a `TidyResult`. Writes UTF-8 to `output` when supplied. |

Bibliography reads try UTF-8, then decode Windows-1252-compatible input and
attach a `text_encoding` diagnostic. Its spans refer to the decoded UTF-8
source. File errors reject the returned promise.

## Errors

[Errors and diagnostics](/reference/errors) defines exception classes,
argument validation, parser diagnostics, and formatting warnings for both bindings.

## Memory lifetime and version

JavaScript garbage collection manages RefKit objects and their WebAssembly
resources. A `Document` retains its library and style, and raw entry and field
views retain their owning bibliography. Returned entry and rendering records
are JavaScript data.

`version` is available during import. After initialization, `getBuildInfo()`
returns `{ version, buildMode, target }`. `buildMode` is `"debug"` or `"release"`,
and `target` identifies the WebAssembly compilation target. Keep the JavaScript
modules and WebAssembly asset from the same package version together when deploying.
