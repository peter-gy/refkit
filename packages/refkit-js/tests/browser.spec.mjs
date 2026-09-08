import { test, expect } from "@playwright/test";

test("browser module initializes concurrently, renders, and edits with CSP", async ({
  page,
}) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/");
  const result = await page.evaluate(async () => {
    const rk = await import("/dist/index.js");
    let beforeInit;
    try {
      rk.Library.parseBibtex("");
    } catch (error) {
      beforeInit = error instanceof rk.RefkitError;
    }
    await Promise.all([rk.init(), rk.init()]);
    const source = "@book{doe,author={Doe, Jane},title={A Book},year={2024}}";
    const library = rk.Library.parseBibtex(source);
    const text = rk.cite(library, "doe").text;
    const raw = rk.BibDocument.parse(source);
    raw.entries.getUnique("doe").fields.getUnique("title").value = "Edited";
    const title = rk.Library.parseBibtex(raw.tidy().bibtex).get("doe").title;
    const structuredError = (() => {
      try {
        rk.Library.parseBibtex("@book{");
      } catch (error) {
        return error instanceof rk.ParseError && error.diagnostics.length > 0;
      }
    })();
    return { beforeInit, text, title, structuredError };
  });
  expect(result.beforeInit).toBe(true);
  expect(result.text).toBe("(Doe, 2024)");
  expect(result.title).toBe("Edited");
  expect(result.structuredError).toBe(true);
  expect(errors).toEqual([]);
});

test("failed initialization can retry from supplied bytes", async ({
  page,
}) => {
  await page.goto("/");
  const result = await page.evaluate(async () => {
    const rk = await import("/dist/index.js");
    let failed = false;
    try {
      await rk.init(new Uint8Array([0, 1]));
    } catch {
      failed = true;
    }
    const bytes = await (
      await fetch("/dist/wasm/refkit_js_native_bg.wasm")
    ).arrayBuffer();
    await rk.init(bytes);
    const parsed = rk.Library.parseBibtex("@book{a,title={A}}");
    const title = parsed.get("a").title;
    return { failed, title };
  });
  expect(result).toEqual({ failed: true, title: "A" });
});

test("module workers use the same browser entry point", async ({ page }) => {
  await page.goto("/");
  const result = await page.evaluate(async () => {
    const source = `import { init, Library } from '${location.origin}/dist/index.js'; await init(); const library = Library.parseBibtex('@book{worker,title={Worker}}'); postMessage(library.keys());`;
    const url = URL.createObjectURL(
      new Blob([source], { type: "text/javascript" }),
    );
    try {
      return await new Promise((resolve, reject) => {
        const worker = new Worker(url, { type: "module" });
        worker.onmessage = (event) => {
          worker.terminate();
          resolve(event.data);
        };
        worker.onerror = (event) => {
          worker.terminate();
          reject(new Error(event.message));
        };
      });
    } finally {
      URL.revokeObjectURL(url);
    }
  });
  expect(result).toEqual(["worker"]);
});

test("bundled npm consumer emits and loads the WebAssembly asset", async ({
  page,
}) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("/bundle/index.html");
  await expect(page.locator("#result")).toHaveText("(Doe, 2024)");
  expect(errors).toEqual([]);
});
