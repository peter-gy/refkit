import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import {
  readLibrary,
  readBibDocument,
  tidyFile,
  RefkitError,
} from "refkit-js/node";

test("file readers select the format and report legacy text decoding", async (t) => {
  const directory = await mkdtemp(join(tmpdir(), "refkit-files-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const input = join(directory, "sources.BIB");
  await writeFile(input, Buffer.from("@book{cafe,title={Caf\xe9}}", "latin1"));
  const library = await readLibrary(pathToFileURL(input));
  assert.equal(library.get("cafe").title, "Café");
  assert.equal(library.diagnostics[0].code, "text_encoding");
  const raw = await readBibDocument(input);
  assert.equal(
    raw.entries.getUnique("cafe").fields.getUnique("title").value,
    "Café",
  );
  assert.equal(raw.diagnostics[0].code, "text_encoding");
  const yaml = join(directory, "items.YAML");
  await writeFile(yaml, "one:\n  type: book\n  title: YAML\n");
  assert.equal((await readLibrary(yaml)).get("one").title, "YAML");
  await assert.rejects(
    readLibrary(join(directory, "absent.bib")),
    (error) => error instanceof RefkitError && error.cause.code === "ENOENT",
  );
});

test("tidyFile writes UTF-8 output and preserves the source file", async (t) => {
  const directory = await mkdtemp(join(tmpdir(), "refkit-files-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const input = join(directory, "sources.bib");
  const output = join(directory, "formatted.bib");
  const original = Buffer.from("@book{cafe,title={Caf\xe9}}", "latin1");
  await writeFile(input, original);
  const result = await tidyFile(input, { output, options: { escape: false } });
  assert.equal(await readFile(output, "utf8"), result.bibtex);
  assert.match(result.bibtex, /Café/);
  assert.deepEqual(await readFile(input), original);
});
