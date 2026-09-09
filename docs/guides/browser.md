---
description: Initialize RefKit in a browser or worker and serve the WebAssembly asset with the required security policy.
---

# Run in a Browser

`refkit-js` loads RefKit's Rust core as [WebAssembly](https://webassembly.org/), a compiled module executed by the browser. Install the package with `npm install refkit-js`, then initialize it before using the [shared bibliography API](/get-started).

## Initialize the module

Use this example in an existing browser application built with a bundler such as [Vite](https://vite.dev/guide/). Put it in the application's TypeScript entry module, for example `src/main.ts`, and start the application with its development command, typically `npm run dev`. The bundler resolves the npm import and serves the compiled module.

Use a browser with WebAssembly and [FinalizationRegistry](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/FinalizationRegistry), the JavaScript facility that releases native resources when objects become unreachable.

```ts
import * as rk from "refkit-js/browser";

await rk.init();
const library = rk.Library.parseBibtex(
  "@book{doe2024, author={Doe, Jane}, title={Browser Citations}, year={2024}}",
);
console.log(rk.cite(library, "doe2024").text);
```

The browser console prints `(Doe, 2024)`. After initialization, parsing, rendering, and editing are synchronous. Concurrent `init()` calls share the loading operation, and later calls reuse the initialized module.

## Load on demand

Importing the browser entry loads the JavaScript API. The first `init()` loads the generated bindings and WebAssembly through a [dynamic import](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Operators/import). Put `init()` in the action that first needs bibliography processing:

```ts
import { init, Library, cite } from "refkit-js/browser";

export async function renderCitation(source: string, key: string): Promise<string> {
  await init();
  return cite(Library.parseBibtex(source), key).text;
}
```

Call `renderCitation(source, key)` from an editor action or a file-selection handler. An application that needs RefKit immediately can await `init()` during startup. An application that can predict demand can call it earlier to preload the engine. Await the same initialization before processing input, and handle its rejection if loading fails. A failed WebAssembly download or compilation can be retried by calling `init()` again.

Keep [code splitting](https://vite.dev/guide/features.html#async-chunk-loading-optimization) enabled so the bundler preserves the deferred import. A build that combines dynamic imports into its entry file pays the embedded payload cost when that file loads. [Vite library builds](https://vite.dev/config/build-options.html#build-assetsinlinelimit) embed assets in JavaScript, so RefKit keeps its binary URL inside the deferred bindings. When publishing a library, keeping `refkit-js` external lets the consuming application own asset emission and caching.

## Serve the WebAssembly asset

The default loader resolves the packaged `.wasm` file relative to its JavaScript module. Bundlers such as [Vite](https://vite.dev/guide/assets.html#new-url-url-import-meta-url) can emit this asset and rewrite its URL. Keep the JavaScript and WebAssembly files from the same package version together.

For an explicit asset location, copy the file exported as `refkit-js/refkit.wasm` to your public assets and initialize with its URL:

```ts
import { init } from "refkit-js/browser";

await init(new URL("/assets/refkit.wasm", location.origin));
```

Serve it over HTTP or HTTPS with the [`application/wasm` media type](https://developer.mozilla.org/en-US/docs/WebAssembly/Reference/JavaScript_interface/instantiateStreaming_static). Cross-origin assets need an appropriate [Cross-Origin Resource Sharing response](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS) from the asset server.

If the application sets a [Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Content-Security-Policy/script-src), allow WebAssembly compilation with `'wasm-unsafe-eval'` in `script-src` and allow the asset origin in `connect-src`. A same-origin policy can include:

```text
script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'
```

## Read user-supplied files

The [File API](https://developer.mozilla.org/en-US/docs/Web/API/File) reads a file selected by the user. Pass its text to the shared parser:

```ts
import { Library } from "refkit-js/browser";

async function readBibliography(file: File) {
  return Library.parseBibtex(await file.text());
}
```

Call this function after initialization. `File.text()` decodes UTF-8. RefKit's Node [file helpers](/reference/javascript#node-filesystem-helpers) also handle Windows-1252-compatible bibliography files.

## Use a worker for longer operations

A [Web Worker](https://developer.mozilla.org/en-US/docs/Web/API/Web_Workers_API/Using_web_workers) runs work off the browser's main thread. Import and initialize RefKit inside each module worker, then return text or ordinary result records through `postMessage`:

```ts
import * as rk from "refkit-js/browser";

await rk.init();
self.onmessage = ({ data }: MessageEvent<string>) => {
  const library = rk.Library.parseBibtex(data);
  self.postMessage(rk.fullBibliography(library));
};
```

Each worker has its own module and bibliography objects. Pass source text or result records between workers. Follow the shared guides for [parsing](/guides/parse-bibliographies), [rendering](/guides/render-citations), [editing](/guides/edit-bibtex), and [formatting](/guides/format-bibtex).

To run the Python binding inside a browser, use [Pyodide](/pyodide).
