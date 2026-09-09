# RefKit for JavaScript

`refkit-js` parses bibliography data, renders citations, and edits
[BibTeX](https://ctan.org/pkg/bibtex) in
[Node.js](https://nodejs.org/) and browsers. It runs RefKit's Rust core through
[WebAssembly](https://webassembly.org/), a portable compiled module loaded by
JavaScript.

Install for Node.js 22.19 or newer:

```sh
npm install refkit-js
```

```js
import { Citation, Document, Library, Style } from "refkit-js";

const library = Library.parseBibtex(
  "@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}",
);
const style = Style.load("apa");
const document = new Document(library, style, { locale: "en-US" });
const result = document.render([new Citation("intro", "doe2024")]);
console.log(result.get("intro").text);
console.log(result.bibliography.text);
```

The citation is `(Doe, 2024)`. Each render call processes the complete ordered
citation list and returns plain text, HTML, and a structured tree.

## Browser initialization

Import the browser entry at startup and call `init()` when a bibliography is needed:

```js
import { init, Library, cite } from "refkit-js/browser";

export async function renderCitation(source, key) {
  await init();
  return cite(Library.parseBibtex(source), key).text;
}
```

Call `renderCitation(source, key)` from the action that needs a citation. Importing
the browser entry loads the JavaScript API. The first `init()` dynamically loads
the generated bindings and WebAssembly. Concurrent calls share that operation,
and subsequent calls reuse it. Parsing and rendering then run synchronously.

Serve the JavaScript modules and bundled `.wasm` asset over HTTP or HTTPS. Your bundler must emit the `.wasm`
asset at its module-relative URL, or pass its deployed URL to `init()`.

A [Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy)
that restricts scripts must permit WebAssembly compilation with
`'wasm-unsafe-eval'` in `script-src`. Allow the asset origin in `connect-src`
and serve `.wasm` as `application/wasm` for streaming compilation.

## Read files in Node.js

```js
import { fullBibliography } from "refkit-js";
import { readLibrary } from "refkit-js/node";

const library = await readLibrary("references.bib");
console.log(fullBibliography(library).text);
```

The Node helpers also expose `readBibDocument`, `readStyle`, and `tidyFile`.
File operations return promises. Parsing, rendering, and raw edits run
synchronously after initialization.

Read the [JavaScript guide](https://peter-gy.github.io/refkit/get-started)
for browser deployment and editing, or the
[JavaScript reference](https://peter-gy.github.io/refkit/reference/javascript)
for the public API and Python name mapping. TypeScript declarations ship with
the package.
