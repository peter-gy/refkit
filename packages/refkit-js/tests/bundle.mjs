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
  const html = (source) =>
    `<!doctype html><html lang="en"><meta charset="utf-8"><title>RefKit bundled consumer</title><body><button id="render">Render citation</button><output id="result">Ready</output><script type="module" src="${source}"></script></body></html>`;
  await writeFile(join(root, "index.html"), html("/main.js"));
  await writeFile(
    join(root, "main.js"),
    `import { init, Library, cite } from 'refkit-js';
export async function render() {
  await init();
  const library = Library.parseBibtex('@book{doe,author={Doe, Jane},title={Book},year={2024}}');
  return cite(library, 'doe').text;
}
document.querySelector('#render').addEventListener('click', async () => {
  document.querySelector('#result').textContent = await render();
});
document.querySelector('#result').textContent = 'Imported';`,
  );
  await build({
    root,
    configFile: false,
    base: "/bundle/",
    build: { target: "es2022", outDir: "bundle" },
    logLevel: "error",
  });
  await build({
    root,
    configFile: false,
    base: "/library/",
    build: {
      target: "es2022",
      outDir: "library",
      lib: {
        entry: join(root, "main.js"),
        formats: ["es"],
        fileName: "refkit",
      },
    },
    logLevel: "error",
  });
  await writeFile(join(root, "library/index.html"), html("/library/refkit.js"));
  return root;
}
