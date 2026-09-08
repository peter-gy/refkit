import { createHash } from "node:crypto";
import { readFile, readdir } from "node:fs/promises";
import { resolve, relative } from "node:path";
import { fileURLToPath } from "node:url";

export const packageRoot = fileURLToPath(new URL("../", import.meta.url));
export async function fingerprint() {
  const root = resolve(packageRoot, "../..");
  const files = [
    "Cargo.toml",
    "Cargo.lock",
    "crates/refkit-core/Cargo.toml",
    "packages/refkit-js/package.json",
    "packages/refkit-js/package-lock.json",
    "packages/refkit-js/tsconfig.json",
    "packages/refkit-js/rust/Cargo.toml",
  ];
  async function collect(directory) {
    for (const item of await readdir(resolve(root, directory), {
      withFileTypes: true,
    })) {
      const path = `${directory}/${item.name}`;
      if (item.name === "wasm") continue;
      if (item.isDirectory()) await collect(path);
      else files.push(path);
    }
  }
  for (const directory of [
    "crates/refkit-core/src",
    "packages/refkit-js/rust/src",
    "packages/refkit-js/src",
    "packages/refkit-js/scripts",
  ])
    await collect(directory);
  const hash = createHash("sha256");
  for (const file of files.sort())
    hash
      .update(relative(root, resolve(root, file)))
      .update("\0")
      .update(await readFile(resolve(root, file)))
      .update("\0");
  return hash.digest("hex");
}
