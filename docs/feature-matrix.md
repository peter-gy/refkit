# Feature Matrix

This matrix compares the refkit workspace at 0.0.2 with the inspected local checkouts of
citeproc-js, citeproc-py, and python-bibtexparser on 2026-06-10. It describes
the features visible in the checked source trees, README files, tests, and
package metadata. Upstream releases may differ.

Evidence paths are relative to the corresponding project root.

The python-bibtexparser feature rows describe the inspected local v2 beta source tree. The benchmark suite installs PyPI `bibtexparser==2.0.0b9` for its comparison rows.

## Status Legend

| Status | Meaning |
| --- | --- |
| Yes | The inspected source exposes the feature as a user-facing contract. |
| Partial | The package covers part of the contract, with the missing scope named in the note. |
| External | The package expects the caller, a companion package, or bundled data to provide the feature. |
| No | No matching user-facing feature was found in the inspected source. |

## Package Roles

| Package | Primary role | Main API shape | Current status from source |
| --- | --- | --- | --- |
| citeproc-js | JavaScript CSL and CSL-M processor. It renders citation clusters and bibliographies from CSL-like item data supplied by a host application. | `new CSL.Engine(sys, style)`, then `processCitationCluster`, `makeCitationCluster`, `makeBibliography`, `updateItems`, and related state APIs. | Mature processor package. `package.json` reports `citeproc` 2.4.63, based on citeproc-js 1.4.63. README states the project passes more than 1,300 integration tests. |
| citeproc-py | Python CSL 1.0.1 processor with JSON and BibTeX source adapters. | `CitationStylesStyle`, `CitationStylesBibliography`, `Citation`, `CitationItem`, source adapters, and formatter modules. | Alpha API by package metadata. README states Python 3.9 and newer, lxml dependency, and almost 60 percent of the relevant citeproc test suite. |
| python-bibtexparser | Python parser and writer for `.bib` documents. It preserves document blocks and supports transformation middleware. | `parse_string`, `parse_file`, `write_string`, `write_file`, `Library`, block classes, middleware, and `BibtexFormat`. | Version 2 beta in README and metadata. It is a BibTeX document library, not a CSL renderer. |
| refkit | Pure Python package backed by the exact matching `refkit-core` native package. It combines normalized citation rendering with a raw BibTeX editing model. | `Library`, `Entry`, `Style`, `Locale`, `Citation`, `Cite`, `CitationGroup`, `Document`, `RenderedDocument`, `Rendered`, `BibDocument`, `cite`, and `full_bibliography`. | Version 0.0.2. Current API supports Python 3.11 to 3.14 and rejects a mismatched `refkit-core` version at import time. |
| refkit-core | Rust/PyO3 extension package used by `refkit`. It builds `refkit_core._refkit_core` for CPython and PyEmscripten. | `Library`, `Entry`, `Style`, `Locale`, `Citation`, `Cite`, `CitationGroup`, `Document`, `RenderedDocument`, `Rendered`, `BibDocument`, and `build_info`. | Version 0.0.2. Current package builds CPython wheels with the Python 3.11 stable ABI and a Python 3.14 Pyodide wheel through `wasm32-unknown-emscripten`. |
| polars-refkit | Polars expression plugin backed by Rust through PyO3, pyo3-polars, and maturin. It parses, inspects, projects, and renders BibTeX rows inside Polars query plans. | `cite`, `cite_each`, `cite_group`, `cite_html`, `full_bibliography_text`, `full_bibliography_html`, `entry_count`, `can_parse`, `has_diagnostics`, `keys`, `entries`, `parse_report`, `diagnostics`, `to_hayagriva_json`, and the `pl.Expr.refkit` namespace. | Version 0.0.2. Current API supports Python 3.11 to 3.14 and returns null for row-level parse failures where the expression has a scalar or list value result. Current package builds CPython wheels with the Python 3.11 stable ABI and a Python 3.14 Pyodide wheel through `wasm32-unknown-emscripten`. |

## High-Level Workflows

| Workflow | citeproc-js | citeproc-py | python-bibtexparser | refkit workspace 0.0.2 |
| --- | --- | --- | --- | --- |
| Render citations from a style and reference database | Yes. It is the core processor workflow. | Yes. It registers citation items against a source and renders through a style. | No. It does not render CSL citations. | Yes. `Document.render` renders named citations from `Library` and `Style`. |
| Render a bibliography | Yes. `makeBibliography` renders registered and uncited items with optional section filters. | Yes. `CitationStylesBibliography.bibliography` renders registered items. | No. It writes BibTeX, not styled bibliographies. | Yes. `Document.render` and `Document.cited_bibliography` render cited entries. `Document.full_bibliography` and top-level `full_bibliography` render the full library. |
| Parse and normalize `.bib` for rendering | External. The host application supplies CSL item data. | Partial. The `BibTeX` source adapter maps BibTeX entries into CSL references. | No normalized CSL model. It parses BibTeX blocks. | Yes. `Library.read` parses `.bib` through Hayagriva. |
| Parse and normalize Hayagriva YAML | No. | No. | No. | Yes. `Library.read` supports `.yaml` and `.yml`. |
| Edit a `.bib` file while preserving comments and malformed blocks | No. | No. | Yes. `Library` stores comments, strings, preambles, failed blocks, raw text, and order. | Partial. `BibDocument` preserves those blocks and can write field edits while keeping unrelated raw blocks intact. Add, remove, reorder, and middleware transforms are outside the current public contract. |
| Use one ergonomic Python package for citation rendering and raw BibTeX repair | No. JavaScript runtime and host app required. | Partial. Citation rendering exists. Raw BibTeX editing is outside its API. | Partial. Raw BibTeX editing exists. Citation rendering is outside its API. | Yes for the current covered surface. `Library` handles normalized rendering and `BibDocument` handles raw editing. |

## Input And Data Model

| Feature | citeproc-js | citeproc-py | python-bibtexparser | refkit workspace 0.0.2 |
| --- | --- | --- | --- | --- |
| CSL item data input | Yes. Items are retrieved through `sys.retrieveItem`. | Yes. `CiteProcJSON` accepts citeproc-js-like JSON. | No. | No public CSL JSON reader yet. |
| Host-supplied item retrieval | Yes. `CSL.Engine` stores `sys` and calls `retrieveItem`. | Partial. Sources are Python mappings that return references by key. | No. | No. `Library` owns parsed entries. |
| BibTeX input | External for rendering. | Yes through `citeproc.source.bibtex.BibTeX`. | Yes through `parse_string` and `parse_file`. | Yes through `Library.read` and `BibDocument.read`. |
| BibLaTeX input | External for rendering. | Partial. The adapter maps a BibTeX-style subset. Full BibLaTeX support is outside the inspected adapter contract. | Partial. Docs state the syntax should work in many cases. The project has not checked all biber and BibLaTeX features. | Yes for normalized `Library.read` through Hayagriva and the `biblatex` Rust crate dependency. Raw `BibDocument` is syntax-oriented. |
| YAML bibliography input | No. | No. | No. | Yes through Hayagriva YAML. |
| Parse from file path | External. The style can be supplied as XML or JSON. Items come from `sys`. | Yes for styles, locales, and BibTeX source files. | Yes. | Yes for `Library.read`, `Style.from_path`, and `BibDocument.read`. |
| Parse from string | Yes for CSL style and locale XML or JSON through `CSL.setupXml`. | Yes for style XML file-like or XML data, and JSON source data. | Yes through `parse_string`. | Yes. `Library.parse_bibtex`, `Library.parse_yaml`, `Style.from_xml`, and `BibDocument.parse` accept in-memory source. |
| Normalized reference object | Yes. CSL item objects are copied and enriched inside processor state. | Yes. `Reference`, `Name`, `Date`, `DateRange`, and `LiteralDate` represent CSL data. | No CSL reference model. | Yes. `Entry` wraps Hayagriva entries and exposes selected typed properties. |
| Raw BibTeX document object | No. | No. | Yes. `Library` stores blocks in insertion order. | Yes. `BibDocument` stores raw blocks with byte spans and entry field handles. |
| Two-model split for normalized rendering and raw editing | External. Rendering state is separate from host data. | Partial. Rendering sources and style objects are separate. Raw BibTeX editing is not modeled. | No. It owns the raw BibTeX model only. | Yes. `Library` is normalized for rendering and selection. `BibDocument` is raw for editing and repair. |

## Raw BibTeX Document Features

| Feature | citeproc-js | citeproc-py | python-bibtexparser | refkit workspace 0.0.2 |
| --- | --- | --- | --- | --- |
| Top-level block order | No. | No. | Yes. `Library.blocks` preserves insertion order. | Yes. `BibDocument.blocks` reports block order. |
| Entry blocks | No. | Partial. BibTeX source reads entries for rendering, not raw editing. | Yes. `Entry` exposes `entry_type`, `key`, `fields`, and `fields_dict`. | Yes. `BibEntry` exposes `key`, `kind`, `fields`, and `span`. |
| Comment blocks | No. | No. | Yes. Explicit and implicit comments are block types. | Yes. `%` comments and `@comment` blocks are preserved as comment blocks. |
| Preamble blocks | No. | Partial. citeproc-py's BibTeX adapter parses preamble macros for LaTeX expansion. It does not expose a raw preamble editing model. | Yes. `Preamble` is a block type. | Yes. `BibDocument.preamble` returns parsed preamble values. |
| String definitions | No. | Partial. The BibTeX adapter uses preamble macros and parsed fields, not raw `@string` editing. | Yes. `String` blocks and `strings_dict` are exposed. | Yes. `BibDocument.strings` returns string definitions. |
| Failed parse blocks | No. | No. | Yes. `ParsingFailedBlock`, `MiddlewareErrorBlock`, and duplicate-key error blocks are exposed through `failed_blocks`. | Yes. `BibDocument.failed_blocks` reports malformed raw blocks with raw text, error, and byte span. |
| Raw text access | No. | No. | Yes. Blocks expose `raw`, with a middleware caveat that it may become stale after transforms. | Partial. `BibDocument.blocks` includes raw text for comments, failed blocks, and other raw text. Entry field writes use the original entry raw span. |
| Source position metadata | No. | Partial. lxml elements and BibTeX internals can carry source lines. No raw document API exposes spans. | Partial. Blocks and fields expose start line. | Yes. Blocks, entries, and fields expose byte spans. |
| Duplicate key handling in raw document | No. | No raw document model. | Yes. Duplicates become `DuplicateBlockKeyBlock` unless configured to raise. | Yes. Raw blocks are preserved on write. Direct `entries[key]` lookup raises when the key is ambiguous, and `entries.get_all(key)` exposes source-order occurrences. |
| Duplicate field handling | No. | No raw document model. | Yes. Duplicate fields can become `DuplicateFieldKeyBlock`. | Yes. Direct `fields[name]` lookup raises when the field is ambiguous, and `fields.get_all(name)` exposes source-order occurrences for targeted edits. |
| Field value edit | No. | No. | Yes through entry field mutation and writer output. | Yes. `BibField.value` updates a field and `BibDocument.write` rewrites only changed field spans. |
| Add or remove fields | No. | No. | Yes. `Entry.set_field`, `pop`, item assignment, and deletion are available. | No current public API. |
| Add, remove, replace, or reorder blocks | No. | No. | Yes. `Library.add`, `remove`, and `replace` exist. Sorting middleware can reorder blocks. | No current public API. |
| Formatting control when writing BibTeX | No. | No. | Yes. `BibtexFormat` controls indentation, value alignment, block separator, trailing comma, and failed-block comment text. | Partial. Field edits preserve original formatting around unchanged spans. No writer formatting object is exposed. |
| LaTeX encode and decode transforms | No. | Partial. The BibTeX adapter parses LaTeX for CSL rendering. | Yes through `LatexEncodingMiddleware` and `LatexDecodingMiddleware`. | No current public API. |
| String interpolation | No. | Partial inside the BibTeX source adapter. | Yes through `ResolveStringReferencesMiddleware`. | Partial. Normalized `Library` delegates to Hayagriva parsing. Raw `BibDocument` preserves definitions without interpolation. |
| Month normalization | No. | Partial inside the BibTeX source adapter. | Yes through month middleware. | Partial through normalized parser behavior. No raw transform API. |
| Name splitting and merging middleware | No. | Partial. The BibTeX adapter has BibTeX name parsing for CSL references. | Yes through `SeparateCoAuthors`, `MergeCoAuthors`, `SplitNameParts`, and `MergeNameParts`. | Partial. Normalized entries expose Hayagriva name handling through rendering. No raw name transform API. |
| Fault-tolerant parsing | No. | Partial. README lists many failing CSL cases. The BibTeX adapter is not a raw recovery parser. | Yes. The splitter recovers and records failed blocks. | Yes. `Library.read(recovery="report")` keeps recoverable normalized entries and records diagnostics. `recovery="error"` is the default. `BibDocument.read` preserves failed raw blocks. |

## CSL Styles And Locales

| Feature | citeproc-js | citeproc-py | python-bibtexparser | refkit workspace 0.0.2 |
| --- | --- | --- | --- | --- |
| CSL style parsing | Yes. `CSL.setupXml` accepts serialized XML, serialized JSON, DOM, E4X, or JS objects. | Yes. `CitationStylesStyle` parses CSL XML through lxml. | No. | Yes. `Style.load`, `Style.from_xml`, and `Style.from_path` use Hayagriva and Citationberg. |
| CSL schema validation | Partial. The repo includes CSL and CSL-M schemata and test infrastructure. Runtime setup accepts parsed style input. | Yes. `CitationStylesXML` validates with RelaxNG when enabled. | No. | Partial. `Style.from_xml` returns a parse error for invalid CSL XML. No public schema-validation switch is exposed. |
| Bundled styles | Partial. Fixtures and style data exist in the repo. The npm package ships `citeproc_commonjs.js` only by package metadata. | Yes. `data/styles/*.csl` are packaged, with optional `citeproc-py-styles` for more styles. | No. | Yes. `Style.load` uses Hayagriva archive styles by name. |
| Dependent style resolution | External. Host code can supply style data. | Partial. `CitationStylesStyle` loads a style by id or path, with extra styles available through `citeproc-py-styles`. | No. | No current support. `Style.from_xml` rejects dependent styles that need parent resolution. |
| Locale loading | Yes. Engine calls `sys.retrieveLocale` and the repo includes CSL locales. | Yes. `CitationStylesLocale` loads bundled locale XML and falls back through style locale resolution. | No. | Yes. `Locale.load` and `Document` use archived Hayagriva locales. |
| Locale fallback | Yes. Locale resolution and style locale sniffing are implemented. | Yes. Style locale list falls back through in-style locales, primary dialect, and `en-US`. | No. | Partial. `Document` accepts a locale code and Hayagriva archived locale data. The public fallback chain is not configurable. |
| Juris-M CSL-M extensions | Yes. README names CSL-M mode, legal content, multilingual content, and jurisdiction modules. | No. README lists citeproc-js extensions such as raw dates, static ordering, and literal names as unsupported. | No. | No current public CSL-M extension API. |
| Jurisdiction modules for legal citations | Yes. `retrieveStyleModule`, `loadStyleModule`, and `juris-modules` support legal module loading. | No. | No. | No current public API. |
| Abbreviations | Yes. `setAbbreviations` and `sys.getAbbreviation` are integrated. | No public abbreviation API found. | No citation abbreviation API. | No current public API. |
| Multilingual item variants | Yes. Language preferences, transliteration, translation, and multilingual name handling are present. | Partial. Locale fallback exists. Multilingual item variant handling was not found as a main API. | No. | Partial through Hayagriva rendering if supported by source data. No public multilingual preference API is exposed. |

## Citation Rendering

| Feature | citeproc-js | citeproc-py | python-bibtexparser | refkit workspace 0.0.2 |
| --- | --- | --- | --- | --- |
| Citation cluster rendering | Yes. `processCitationCluster`, `appendCitationCluster`, `previewCitationCluster`, and `makeCitationCluster` exist. | Yes. `Citation` and `CitationItem` are rendered through `CitationStylesBibliography.cite`. | No. | Yes. `Document.render` accepts named `Citation` objects. `CitationGroup` renders several items as one citation. `polars-refkit` exposes `cite_each` for one citation per key and `cite_group` for one grouped citation. |
| Citation item locators | Yes. Locator parsing and labels are supported. | Partial. `CitationItem` accepts `locator`, `prefix`, and `suffix`, while locator labels are modeled separately in `Locator`. | No. | Yes. `Cite(key, *, locator, label)` maps labels to CSL locators. |
| Citation item prefix and suffix | Yes. CSL item data can carry affixes. | Yes. `CitationItem` optional arguments include `prefix` and `suffix`. | No. | No current `Cite` prefix or suffix API. |
| Citation sorting inside a cluster | Yes. `makeCitationCluster` computes citation sort keys when the style defines citation sorting. | Yes through style layout processing. | No. | Yes through Hayagriva rendering. |
| Bibliography sorting | Yes. `updateItems` and registry sort keys drive bibliography order. | Yes. `sort_bibliography` and `CitationStylesBibliography.sort` exist. | No citation sorting. | Yes through Hayagriva rendering. |
| Disambiguation | Yes. Dedicated disambiguation state and registry logic are implemented. | Partial. README lists several disambiguation and year-suffix features as missing. | No. | Partial through Hayagriva. refkit does not expose a separate disambiguation control API. |
| Numeric citation numbering | Yes. Registry renumbering and numeric output mode are implemented. | Yes for supported CSL styles. | No. | Yes through CSL styles such as IEEE. |
| Citation collapsing | Yes. citeproc-js handles collapse behavior. | No. README lists collapsing as missing. | No. | Partial through Hayagriva. Full CSL test-suite parity is not claimed by refkit. |
| Et-al subsequent settings | Yes. Options exist for first and subsequent references. | No. README lists subsequent et-al settings as missing. | No. | Partial through Hayagriva. No public per-style override is exposed. |
| Punctuation in quote | Yes. citeproc-js has locale and punctuation behavior. | No. README lists `punctuation-in-quote` as missing. | No. | Partial through Hayagriva style rendering. No public override is exposed. |
| Display attributes | Yes. Output modes handle display classes such as block, left margin, right inline, and indent. | No. README lists `display` as missing. | No. | Yes for rendered HTML tree output where Hayagriva display metadata is present. |
| Raw dates extension | Yes. README names raw dates as a citeproc-js extension. | No. README lists raw dates as unsupported. | No. | No current public extension API. |
| Static ordering extension | Yes. README names static ordering as a citeproc-js extension. | No. README lists static ordering as unsupported. | No. | No current public extension API. |
| Literal names extension | Yes. README names literal names as a citeproc-js extension. | No. README lists literal names as unsupported. | No. | Partial. Hayagriva data may represent names. refkit exposes no citeproc-js literal-name extension API. |
| Missing reference behavior | External by host data retrieval and processor errors. | Partial. `register` can call a callback for missing items. | No citation references. | Yes. `Document.render` raises `MissingReferenceError` when any named citation references a missing key. |

## Dynamic Document State

| Feature | citeproc-js | citeproc-py | python-bibtexparser | refkit workspace 0.0.2 |
| --- | --- | --- | --- | --- |
| Append citation to current document | Yes. `appendCitationCluster` computes prior citation state and appends. | Partial. Calls to `register` and `cite` accumulate registered items. | No. | No current public append API. `Document.render` takes the complete citation list so prior citation outputs and bibliography state are produced together. |
| Insert or edit citation with pre and post citation context | Yes. `processCitationCluster` accepts `citationsPre` and `citationsPost`. | No direct equivalent found. | No. | No current public API. |
| Preview citation without mutating state | Yes. `previewCitationCluster` saves and restores state. | No direct equivalent found. | No. | No current public API. |
| Rebuild processor state from existing document citations | Yes. `rebuildProcessorState` exists for dynamic applications. | No direct equivalent found. | No. | No current public API. |
| Restore saved processor state | Yes. `restoreProcessorState` exists and is marked deprecated in source comments. | No direct equivalent found. | No. | No current public API. |
| Track uncited items in bibliography | Yes. `updateUncitedItems` and `rebuildProcessorState` accept uncited item IDs. | Partial. Bibliography items are those registered through citations. | No. | Partial. `full_bibliography` renders every library key. `Document` has no explicit uncited item API. |
| Bibliography section filtering | Yes. `makeBibliography` supports include, exclude, select, quash, and paged returns. | No public equivalent found. | No. | Partial. `Library.select` filters entries before document construction. `Document.cited_bibliography` and `Document.full_bibliography` have no section filter argument. |
| Paged bibliography output | Yes. `page_start`, `page_length`, and `done` are handled. | No. | No. | No. |

## Output Formats And Rendered Shape

| Feature | citeproc-js | citeproc-py | python-bibtexparser | refkit workspace 0.0.2 |
| --- | --- | --- | --- | --- |
| Plain text citation output | Yes. `text` output mode exists. | Yes. `formatter.plain` exists. | No citation output. | Yes. `Rendered.text` and `to_text`. |
| HTML citation output | Yes. `html` output mode exists. | Yes. `formatter.html` exists. | No citation output. | Yes. `Rendered.html` and `to_html`. |
| RTF output | Yes. `rtf` output mode exists. | No. | No. | No. |
| reStructuredText output | No direct RST mode found. | Yes. `formatter.rst` exists. | No. | No. |
| AsciiDoc output | Yes. `asciidoc` output mode exists. | No. | No. | No. |
| Formatting Objects output | Yes. `fo` output mode exists. | No. | No. | No. |
| Structured render tree | Internal output queues and blobs exist. The public API returns strings and parameter arrays. | No public tree output found. | No citation render tree. | Yes. `Rendered.tree` and `to_tree` return inspectable Python data. |
| HTML escaping | Yes. HTML mode escapes text. | Yes. HTML formatter escapes text. | No rendered HTML. | Yes. Tests cover escaped text and unsafe link filtering. |
| DOI and URL link rendering | Yes. HTML output has DOI and URL wrappers. | Partial. Formatter handles text styles. URL and DOI behavior depends on style rendering. | No. | Yes for links emitted by Hayagriva. Unsafe href schemes are not emitted as links. |
| Bibliography second-field alignment | Yes. HTML output supports left margin and right inline display. | Partial. README lists display as missing, and second-field behavior appears in model defaults. | No. | Yes. IEEE bibliography text and tree include first-field labels. HTML emits left and right margin divs when present. |
| Caller-supplied citation wrappers | Yes. `sys.wrapCitationEntry`, `sys.embedBibliographyEntry`, and `sys.variableWrapper` are integrated. | No direct equivalent found. | No. | No current public API. |

## Querying, Conversion, And Export

| Feature | citeproc-js | citeproc-py | python-bibtexparser | refkit workspace 0.0.2 |
| --- | --- | --- | --- | --- |
| Select entries by bibliography conditions | Partial. Bibliography section filters apply during rendering. | No general query language found. | Partial. Users can iterate blocks and entries in Python. | Yes. `Library.select` accepts Hayagriva selectors such as `article > periodical[volume]`. |
| Retrieve keys | Yes through host item lists and registry state. | Yes. `CitationStylesBibliography.keys` tracks registered keys. | Yes. `entries_dict` and block APIs expose keys. | Yes. `Library.keys`, `BibEntryMap.unique_keys`, and `BibEntryMap.occurrence_keys`. |
| Iterate entries | Partial through registry and host data. | Partial through source mappings and registered items. | Yes. `library.entries`. | Yes. `Library.values` and `BibDocument.entries`. |
| Export normalized entries to dictionaries | No public bulk export found. | Partial. Sources are Python objects and mappings. | No CSL normalization. | Yes. `Library.to_dicts`. |
| Export CSL JSON | Native item data uses CSL JSON-like objects. No package-level exporter was found. | Partial. `CiteProcJSON` imports JSON. No exporter found. | No. | Partial. `polars_refkit.to_hayagriva_json` returns normalized Hayagriva entry JSON with `id` and `key` fields for dataframe rows. The core `refkit` package has no full citeproc-js CSL JSON interchange API. |
| Export Arrow | No. | No. | No. | No current public API. |
| Write `.bib` output | No. | No. | Yes. `write_string` and `write_file`. | Partial. `BibDocument.write` writes field edits while preserving unrelated raw blocks. Normalized `Library.write` is not exposed. |
| Inspect and render BibTeX rows inside Polars | No. | No. | No. | Yes through `polars-refkit`. `entry_count`, `keys`, `entries`, `parse_report`, `diagnostics`, `cite`, and `full_bibliography_html` run as Polars expressions. Rendered structs expose text and HTML together. |

## Extensibility And Integration

| Feature | citeproc-js | citeproc-py | python-bibtexparser | refkit workspace 0.0.2 |
| --- | --- | --- | --- | --- |
| Host integration hooks | Yes. `sys` supplies items, locales, abbreviations, style modules, wrappers, and comparison functions. | Partial. Source mappings and callback-based missing references are extension points. | No renderer integration hooks. | Partial. Python API exposes objects and exceptions. No callback hooks are used in the main API. |
| Middleware stack | No general Python-style middleware stack. The processor has many internal extension points. | No general middleware stack. | Yes. Parse and write stacks accept middleware. | No current public middleware stack. |
| Custom output formatter | Yes by adding output format specs to `CSL.Output.Formats`. | Yes. Formatter modules define style wrappers and can be swapped. | Writer formatting only. | No current public formatter API. |
| Polars expression plugin | No. | No. | No. | Yes through `polars-refkit`, which registers Rust Polars expressions for BibTeX string columns. |
| Type information for Python consumers | No. JavaScript package. | No type stubs found in inspected source. | Yes. `py.typed` is packaged. | Yes. `__init__.pyi` and typed package metadata are present. |
| Rust-backed parsing and rendering | No. | No. | No. | Yes. Native module uses PyO3, maturin, Hayagriva, and BibLaTeX crates. |
| GIL release for heavy work | Not applicable. | No. Pure Python and lxml. | No. Pure Python. | Yes for current heavy paths. `Library.read`, `Document.render`, `Document.cited_bibliography`, `Document.full_bibliography`, rendered tree serialization, and `BibDocument.write` detach after Python inputs are converted to Rust-owned state. |
| Python runtime dependencies | No. JavaScript runtime. | No. Requires lxml. | No. Requires pylatexenc. | Partial. `refkit` has no Python runtime dependencies. `polars-refkit` depends on Polars because it is a Polars expression plugin. |

## Errors, Diagnostics, And Validation

| Feature | citeproc-js | citeproc-py | python-bibtexparser | refkit workspace 0.0.2 |
| --- | --- | --- | --- | --- |
| Structured missing-reference error | External or processor error depending on host behavior. | Partial. Missing items can be passed to a callback during registration. | No citation references. | Yes. `MissingReferenceError`. |
| Parser diagnostics | Yes for processor style errors and test runner output. | Partial. CSL validation warnings and Python exceptions. | Yes. Failed blocks carry exceptions and raw block data. | Yes. `Library.read(recovery="report")` returns recovery diagnostics, and `BibDocument.failed_blocks` preserves raw errors. |
| Test suite against CSL fixtures | Yes. README reports more than 1,300 integration tests. | Yes. README reports almost 60 percent of relevant citeproc tests and tracks expected failures. | No CSL tests. It has parser, writer, and middleware tests. | Partial. Public examples and regression tests exist. Full CSL suite parity is not claimed. |
| Raw BibTeX parser tests | No. | Partial through examples and source adapter tests. | Yes. Splitter, model, library, writer, and middleware tests exist. | Yes. Public tests cover raw preservation, malformed blocks, spans, duplicate keys, field edits, and delimiter safety. |
| Coverage gate | No coverage gate found in inspected metadata. | Coveralls badge exists in README. | Test extras include pytest-cov. No hard coverage gate was found in setup metadata. | Yes. `pytest` is configured with branch coverage and `--cov-fail-under=100`. |

## Packaging And Compatibility

| Feature | citeproc-js | citeproc-py | python-bibtexparser | refkit workspace 0.0.2 |
| --- | --- | --- | --- | --- |
| Package language | JavaScript. | Python. | Python. | Python plus Rust. |
| Package name from metadata | `citeproc`. | `citeproc-py`. | `bibtexparser`. | `refkit` and `polars-refkit`. |
| Import name | `CSL` from bundled JS or CommonJS module. | `citeproc`. | `bibtexparser`. | `refkit` and `polars_refkit`. |
| Version from inspected metadata | 2.4.63. | Versioneer-managed. | 2.0.0b9. | 0.0.2. |
| Python version support | Not applicable. | Python 3.9 and newer, classifiers through 3.13. | Python 3.9 and newer, classifiers through 3.12. | Python 3.11 to 3.14. |
| Refkit workspace license | Not applicable. | Not applicable. | Not applicable. | Apache-2.0. |
| Build system | JavaScript package and repo build scripts. | setuptools with versioneer and schema conversion. | setuptools. | uv workspace with maturin, PyO3, and pyo3-polars. |

## What refkit Unifies Today

refkit already covers the main overlap that requires two Python packages today:

| User workflow | Existing split | refkit path |
| --- | --- | --- |
| Read a `.bib` file and render citations | citeproc-py can render from a BibTeX adapter, while python-bibtexparser can parse raw BibTeX without rendering. | `Library.read`, `Style.load`, `Document.render`, and `Document.cited_bibliography`. |
| Render from a normalized bibliography format | citeproc-py expects CSL JSON or BibTeX sources. python-bibtexparser has no normalized citation renderer. | `Library.read` supports `.bib`, `.yaml`, and `.yml`. |
| Inspect rendered output as text, HTML, and structured data | citeproc-py supports strings in text, HTML, and RST. citeproc-js returns strings and bibliography metadata arrays. | `Rendered.text`, `Rendered.html`, and `Rendered.tree`. |
| Repair a `.bib` title while keeping comments and malformed blocks | python-bibtexparser can preserve and write blocks. citeproc-py does not expose raw editing. | `BibDocument.read`, field mutation through `BibField.value`, and `BibDocument.write`. |
| Query normalized parent relationships | citeproc-py and python-bibtexparser expose Python iteration. No selector language was found. | `Library.select("article > periodical[volume]")`. |
| Inspect or render BibTeX rows in a dataframe | Existing packages require Python object materialization before dataframe use. | `polars_refkit.keys("bibtex")`, `polars_refkit.entry_count("bibtex")`, `polars_refkit.entries("bibtex")`, `polars_refkit.cite("bibtex", "key")`, and `polars_refkit.full_bibliography_html("bibtex")` run as Polars expressions. |

## Migration Paths

The [migration guide](migration.md) gives concrete replacements for common citeproc-py rendering flows and python-bibtexparser raw repair flows. The [API contracts guide](api-contracts.md) defines one-off helper inputs, structured return shapes, raw block records, and public errors.

## Current refkit Gaps

Use the reference package named in the last column when a workflow needs one of these contracts.

| Area | Gap | Reference package with broader coverage |
| --- | --- | --- |
| Dynamic word-processor workflows | Citation IDs, insert-before and insert-after context, preview without mutation, state rebuild, uncited item APIs, and paged bibliography output. | citeproc-js. |
| CSL-M and legal citation extensions | Jurisdiction modules, abbreviation hooks, multilingual preferences, and CSL-M extension APIs. | citeproc-js. |
| Full CSL compatibility claim | Full citeproc-test-suite parity. Refkit uses Hayagriva and covers smoke plus regression behavior. | citeproc-js is the strongest reference from inspected docs. |
| Raw BibTeX transform pipeline | Add, remove, reorder, formatting presets, middleware, LaTeX transforms, month transforms, and name transforms. Refkit preserves raw blocks and edits existing field values. | python-bibtexparser. |
| CSL JSON import and export | `from_csl_json` and `to_csl_json` style workflows. | citeproc-py imports citeproc-js-like JSON. citeproc-js consumes host-supplied CSL item objects. |
| Output formats | RTF, AsciiDoc, Formatting Objects, and reStructuredText. Refkit exposes text, HTML, and tree. | citeproc-js for RTF, AsciiDoc, and FO. citeproc-py for RST. |
| Prefix and suffix on cite items | Prefix and suffix fields on individual cite items. Refkit `Cite` supports key, locator, and label. | citeproc-js and citeproc-py. |

## Evidence Map

| Project | Evidence paths inspected |
| --- | --- |
| citeproc-js | `README.rst`, `package.json`, `src/build.js`, `src/system.js`, `src/api_control.js`, `src/api_update.js`, `src/api_cite.js`, `src/api_bibliography.js`, `src/formats.js`, `src/state.js`, `src/util_modules.js`, `demo/demo.js`, and `src/test_runner.js`. |
| citeproc-py | `README.md`, `setup.py`, `setup.cfg`, `citeproc/frontend.py`, `citeproc/model.py`, `citeproc/source/__init__.py`, `citeproc/source/json.py`, `citeproc/source/bibtex/bibtex.py`, `citeproc/formatter/plain.py`, `citeproc/formatter/html.py`, `citeproc/formatter/rst.py`, and `tests/failing_tests.txt`. |
| python-bibtexparser | `README.md`, `setup.py`, `bibtexparser/__init__.py`, `bibtexparser/entrypoint.py`, `bibtexparser/library.py`, `bibtexparser/model.py`, `bibtexparser/splitter.py`, `bibtexparser/writer.py`, `bibtexparser/middlewares`, `docs/source/quickstart.rst`, `docs/source/customize.rst`, `docs/source/biber.rst`, and parser, writer, model, library, splitter, and middleware tests. |
| refkit workspace | `README.md`, `pyproject.toml`, `Cargo.toml`, `Cargo.lock`, `crates/refkit-core/src/lib.rs`, `crates/refkit-core/src/document.rs`, `crates/refkit-core/src/library/`, `crates/refkit-core/src/raw.rs`, `crates/refkit-core/src/render/`, `crates/refkit-core/src/render_tree.rs`, `crates/refkit-core/src/style.rs`, `crates/refkit-core/src/strings.rs`, `packages/refkit-core/rust/src/`, `packages/refkit-core/rust/src/raw.rs`, `packages/refkit-core/rust/src/rendered.rs`, `packages/polars-refkit/rust/src/expressions/`, `packages/refkit/pyproject.toml`, `packages/refkit/src/refkit/__init__.py`, `packages/refkit/src/refkit/__init__.pyi`, `packages/refkit-core/pyproject.toml`, `packages/refkit-core/src/refkit_core/__init__.py`, `packages/refkit-core/src/refkit_core/_refkit_core.pyi`, `packages/refkit/tests/test_public_api.py`, `packages/polars-refkit/pyproject.toml`, `packages/polars-refkit/polars_refkit/__init__.py`, and `packages/refkit-bench/src/refkit_bench/runner.py`. |
