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

test("browser resolution returns expanded fields and typed parse failures", async ({
  page,
}) => {
  await page.goto("/");
  const result = await page.evaluate(async () => {
    const rk = await import("/dist/index.js");
    await rk.init();
    const document = rk.BibDocument.parse(
      '@string{host="https://example.test/"}\n@misc{entry,url=host # {paper}}',
    );
    const records = document.resolve();
    try {
      rk.BibDocument.parse("@misc{entry,title=undefined}").resolve();
    } catch (error) {
      return {
        records,
        typedError: error instanceof rk.ParseError,
        diagnostic: error.diagnostics[0].code,
      };
    }
    throw new Error("Unresolved string was accepted");
  });
  expect(result).toEqual({
    records: [
      {
        key: "entry",
        entryType: "misc",
        fields: { url: "https://example.test/paper" },
      },
    ],
    typedError: true,
    diagnostic: "unknown_abbreviation",
  });
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

for (const mode of ["bundle", "library"]) {
  test(`${mode} consumer defers engine transfer until use`, async ({
    page,
  }) => {
    const errors = [];
    const responses = [];
    const requests = [];
    page.on("request", (request) => requests.push(request.url()));
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("response", (response) => responses.push(response));
    await page.goto(`/${mode}/index.html`);
    await expect(page.locator("#result")).toHaveText("Imported");
    const startup = await Promise.all(
      responses
        .filter((response) => response.request().resourceType() === "script")
        .map(async (response) => (await response.body()).byteLength),
    );
    expect(startup.length).toBeGreaterThan(0);
    expect(startup.reduce((sum, bytes) => sum + bytes, 0)).toBeLessThan(
      100_000,
    );
    expect(
      requests.some((url) => new URL(url).pathname.endsWith(".wasm")),
    ).toBe(false);
    const before = responses.length;
    await page.getByRole("button", { name: "Render citation" }).click();
    await expect(page.locator("#result")).toHaveText("(Doe, 2024)");
    expect(responses.length).toBeGreaterThan(before);
    expect(errors).toEqual([]);
  });
}

test("browser import defers bindings and compilation until initialization", async ({
  page,
}) => {
  const requests = [];
  page.on("request", (request) => requests.push(request.url()));
  await page.goto("/");
  await page.evaluate(() => {
    const instantiate = WebAssembly.instantiate;
    const streaming = WebAssembly.instantiateStreaming;
    window.compilations = 0;
    WebAssembly.instantiate = (...args) => {
      window.compilations++;
      return instantiate(...args);
    };
    WebAssembly.instantiateStreaming = (...args) => {
      window.compilations++;
      return streaming(...args);
    };
  });
  const imported = await page.evaluate(async () => {
    const rk = await import("/dist/index.js");
    return { key: new rk.Cite("a").key, compilations: window.compilations };
  });
  expect(imported).toEqual({ key: "a", compilations: 0 });
  expect(requests.some((url) => /refkit_js_native/.test(url))).toBe(false);
  const initialized = await page.evaluate(async () => {
    const rk = await import("/dist/index.js");
    const first = rk.init();
    const shared = first === rk.init();
    await first;
    await rk.init();
    return {
      shared,
      compilations: window.compilations,
      version: rk.getBuildInfo().version === rk.version,
    };
  });
  expect(initialized).toEqual({ shared: true, compilations: 1, version: true });
  expect(requests.filter((url) => url.endsWith(".wasm"))).toHaveLength(1);
});

test("failed fetch is shared and can retry a promised asset URL", async ({
  page,
}) => {
  await page.goto("/");
  const result = await page.evaluate(async () => {
    const rk = await import("/dist/index.js");
    const first = rk.init("/missing.wasm");
    const shared = first === rk.init();
    const failed = await first.then(
      () => false,
      () => true,
    );
    await rk.init({
      module_or_path: Promise.resolve(
        new URL("/dist/wasm/refkit_js_native_bg.wasm", location.origin),
      ),
    });
    return {
      shared,
      failed,
      keys: rk.Library.parseBibtex("@book{a,title={A}}").keys(),
    };
  });
  expect(result).toEqual({ shared: true, failed: true, keys: ["a"] });
});

test("rejected input is handled while the binding chunk loads", async ({
  page,
}) => {
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  let release;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  await page.route("**/wasm/refkit_js_native.js", async (route) => {
    await gate;
    await route.continue();
  });
  await page.goto("/");
  try {
    const result = await page.evaluate(async () => {
      const rk = await import("/dist/index.js");
      return rk.init(Promise.reject(new Error("Input unavailable"))).then(
        () => "unexpected success",
        (error) => error.message,
      );
    });
    expect(result).toBe("Input unavailable");
  } finally {
    release();
  }
  await page.evaluate(async () => {
    const rk = await import("/dist/index.js");
    await rk.init();
  });
  expect(errors).toEqual([]);
});
