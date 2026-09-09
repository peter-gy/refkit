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
| `refkit-js/browser` | Shared API with explicit asynchronous initialization. |
| `refkit-js/node` | Shared API and asynchronous filesystem helpers for Node.js. |
| `refkit-js/refkit.wasm` | Packaged WebAssembly asset for bundlers and deployment tooling. |

### `init(input?)`

Loads the packaged WebAssembly module and returns a promise. With no argument,
the browser loader resolves the packaged `.wasm` file relative to its module.
Pass a URL for a separately served asset. `InitOptions` also accepts a request,
response, byte buffer, compiled `WebAssembly.Module`, a promise for those inputs,
or a `{ module_or_path: input }` object. Concurrent calls share initialization.
Calls after successful initialization resolve immediately. Await initialization before calling
the browser API. Read the [browser deployment requirements](/guides/browser#initialize-the-module)
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

### `Library`

| Member | Contract |
| --- | --- |
| `diagnostics` | Structured parser diagnostics. |
| `size` | Number of normalized entries. |
| `keys()` / `values()` | Keys or `Entry` records in library order. |
| `get(key)` | `Entry` or `null`. |
| `getMany(keys)` | Entries in requested order, including repeated keys. Throws for a missing key. |
| `has(key)` / `isEmpty()` | Membership or emptiness. |
| `select(selector)` | Entries matched by a [selector](/reference/selectors). |
| `project(fields?, { keys } = {})` | Records containing requested fields, optionally restricted to ordered keys. |

`project()` defaults to `key`, `title`, `doi`, and `volume`. Supported fields
are `key`, `entryType`, `type`, `title`, `date`, `doi`, and `volume`. `entryType`
and `type` select the normalized entry type and preserve the requested field
name. Missing selected keys throw. A missing field value is `null`.

`Entry` records expose `key`, `entryType`, `title`, `date`, `doi`, `volume`, and
`parents`. `entryType` uses normalized TitleCase names such as `Book`. `parents` is an array of `Entry` records. `title`, `date`, `doi`, and
`volume` can be `null`. `volume` uses the entry's own volume or its first
parent's volume. A `Library` is iterable over its `Entry` records.

## Styles and citation inputs

### `Style.load(name)` and `Style.fromXml(xml)`

Return a prepared [Citation Style Language](https://citationstyles.org/)
style, which controls citation and bibliography formatting. Bundled lookup is
case-insensitive. Custom XML must describe an independent style. Invalid XML,
invalid macro graphs, dependent styles, and unknown bundled names throw.

`title` is the style title. `id` is the requested bundled name or `"xml"` for
an XML input.

### `Locale.load(code)`

Validates a bundled locale and returns a `Locale` with its `code` property.

### `new Cite(key, { locator = null, label = null } = {})`

Creates a citation item. A locator with an omitted label uses `page`. A label
has a rendering effect when a locator is supplied. Invalid labels throw at
render time.

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

Creates a raw bibliography with live entry and field views. Malformed blocks
remain available through `failedBlocks`.

| Member | Contract |
| --- | --- |
| `entries` | Live `BibEntryMap`. |
| `diagnostics` | Source decoding diagnostics. |
| `comments` | Source-order comment strings. |
| `preamble` | Preamble values joined with ` # `. |
| `strings` | String definitions keyed by name. |
| `failedBlocks` / `blocks` | Failed blocks or every source-order block. |
| `toBibtex()` | Serializes the current edits. |
| `resolve()` | Returns detached `ResolvedBibEntry` records with expanded source fields. |
| `tidy({ options } = {})` | Formats the current edits into a `TidyResult`. |

`BibEntry` exposes `key`, `kind`, `fields`, and `span`. `BibField` exposes
`name`, mutable `value`, and `span`. Assigning `value` validates the original
field delimiter before updating the document. `tidy()` returns a formatted
result and preserves the current raw document.

`resolve()` returns source-ordered records with `key`, `entryType`, and `fields`.
It uses the current field edits and raises `ParseError` for invalid or ambiguous
input. See [field resolution](/guides/edit-bibtex#resolve-fields-for-inspection)
for macro handling and diagnostics.

### `BibEntryMap` and `BibFieldMap`

| Member | Contract |
| --- | --- |
| `size` | Number of source occurrences, including duplicates. |
| `uniqueKeys()` | Distinct keys. |
| `occurrenceKeys()` / `occurrences()` | Keys or live views in source order. |
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
