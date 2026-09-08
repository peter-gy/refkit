---
description: Understand why RefKit separates normalized bibliography data, raw BibTeX, and styled output.
---

# Why RefKit

Bibliography work often crosses three different representations: source text that people edit, normalized records that programs query, and styled output that documents display. RefKit gives each representation one owner and keeps their transitions in a portable Rust core.

## Keep source and meaning separate

A normalized entry is convenient for selection and rendering. It cannot preserve every comment, preamble, string definition, malformed block, delimiter, or duplicate occurrence from a `.bib` file.

RefKit exposes both contracts:

- `Library` owns normalized entries and parser diagnostics.
- `BibDocument` owns source-order raw BibTeX and edit-preserving writeback.

The split makes the tradeoff explicit at construction time. A render workflow starts with `Library`. A source repair starts with `BibDocument`.

## Render a complete citation operation

Citation styles can sort items, disambiguate names and years, and change a bibliography based on the citations that appear in a document. `Document.render` accepts the complete ordered citation list for one operation and returns named citations with their cited bibliography.

Separate `Document.render` calls create separate render state. This keeps one result reproducible from its library, style, locale, and ordered citations.

## Inspect rendered structure

Every `Rendered` value exposes plain text, HTML, and a structured tree. The tree keeps links, formatting, display metadata, and bibliography entry identity available to another renderer without parsing HTML.

## Process columns without Python object loops

`polars-refkit` registers Rust-backed expressions on `pl.Expr.refkit`. Each bibliography source row becomes an independent parse and render boundary inside an eager query or lazy plan.

The Polars interface returns native scalar, list, and struct columns. Report expressions keep parser or formatter details next to the row that produced them.

## Run the same core in several hosts

The Rust core accepts in-memory values and returns RefKit-owned records. Python owns paths, objects, and exceptions. TypeScript exposes the same objects through WebAssembly in Node.js and browsers. Polars owns expression registration, broadcasting, dtypes, and row failures. PyEmscripten wheels carry the Python interfaces into Pyodide.

This ownership model keeps bibliography behavior in one implementation while each host exposes native inputs and outputs.

Read [How RefKit Works](/concepts/how-refkit-works) for the object and state model.
