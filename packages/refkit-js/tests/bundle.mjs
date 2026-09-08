import { cp, mkdtemp, mkdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { build } from "vite";

export async function bundleConsumer(packageRoot) {
  const root = await mkdtemp(join(tmpdir(), "refkit-browser-consumer-"));
  const dependency = join(root, "node_modules/refkit-js");
  await mkdir(dependency, { recursive: true });
  await cp(join(packageRoot, "dist"), join(dependency, "dist"), {
    recursive: true,
  });
  await cp(join(packageRoot, "package.json"), join(dependency, "package.json"));
  await writeFile(
    join(root, "package.json"),
    JSON.stringify({ private: true, type: "module" }),
  );
  await writeFile(
    join(root, "index.html"),
    '<!doctype html><html lang="en"><meta charset="utf-8"><title>RefKit bundled consumer</title><body><output id="result">Loading</output><script type="module" src="/main.js"></script></body></html>',
  );
  await writeFile(
    join(root, "main.js"),
    `import { init, Library, cite } from 'refkit-js';
await init();
const library = Library.parseBibtex('@book{doe,author={Doe, Jane},title={Book},year={2024}}');
document.querySelector('#result').textContent = cite(library, 'doe').text;`,
  );
  await build({
    root,
    configFile: false,
    base: "/bundle/",
    build: { target: "es2022", outDir: "bundle" },
    logLevel: "error",
  });
  return root;
}
