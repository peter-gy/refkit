# JavaScript Adapter

`refkit-js` exposes the portable Rust core through one WebAssembly module for Node.js and browsers.

- Keep bibliography semantics in `refkit-core`. The package-local Cargo workspace and lockfile in `rust/` own native dependency resolution. Rust code owns conversion to JavaScript data and calls the public core API.
- Keep browser imports lightweight. `runtime.ts` owns the shared initialization state and dynamically imports generated bindings at `init()`. Shared wrappers use type-only native imports. Keep the default binary URL in the generated loader so library bundlers defer embedded assets with that chunk. Node passes its statically imported bindings into the same runtime owner.
- Keep the public TypeScript API in `src/index.ts`. `src/node.ts` owns filesystem access and automatic Node initialization.
- Preserve Python capability and result parity with camelCase names, options objects, nullable values, and typed records. Update shared parity checks when changing a contract.
- Keep raw entry and field wrappers attached to their original document and occurrence IDs. Native allocations follow garbage collection, and dependent wrappers retain the state they need.
- Generate `src/wasm` and `dist` with `npm run build`. Pin the binding generator to the Rust dependency version. Ship the WebAssembly binary, declarations, and project license in the npm tarball.
- Keep npm package and lock versions aligned with the Rust workspace. Publish the tested tarball through `publish.yml` with the `npm` environment.

Run `make js-check` from the repository root after installing the browser runtimes described in the developer packaging guide. Test the installed tarball, browser loading, worker execution, and TypeScript imports when changing exports or build output.
