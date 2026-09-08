import assert from "node:assert/strict";
import { readFile, stat } from "node:fs/promises";
import { resolve } from "node:path";
import { fingerprint, packageRoot } from "./source.mjs";

const manifest = JSON.parse(
  await readFile(resolve(packageRoot, "package.json"), "utf8"),
);
const build = JSON.parse(
  await readFile(resolve(packageRoot, "dist/build.json"), "utf8"),
);
assert.equal(
  build.source,
  await fingerprint(),
  "Build inputs changed. Run npm run build before packing.",
);
async function checkExports(value) {
  if (typeof value === "string")
    assert.ok(
      (await stat(resolve(packageRoot, value))).isFile(),
      `Missing export ${value}`,
    );
  else for (const entry of Object.values(value)) await checkExports(entry);
}
await checkExports(manifest.exports);
const wasm = await readFile(
  resolve(packageRoot, "dist/wasm/refkit_js_native_bg.wasm"),
);
assert.equal(WebAssembly.validate(wasm), true, "Invalid WebAssembly module");
const sdk = await import("../dist/node.js");
assert.equal(
  sdk.version,
  manifest.version,
  "JavaScript and WebAssembly versions differ",
);
console.log(`Validated ${manifest.name}@${manifest.version}`);
