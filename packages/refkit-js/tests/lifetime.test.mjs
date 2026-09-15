import assert from "node:assert/strict";
import { test } from "node:test";
import { BibDocument, Citation, Document, Library, Style } from "refkit-js";

test("documents and fields retain their native state across garbage collection", async () => {
  const document = new Document(
    Library.parseBibtex("@book{a,author={Doe, Jane},title={A},year={2024}}"),
    Style.load("apa"),
  );
  const [before, after] = (() => {
    const raw = BibDocument.parse("@book{raw,title={Original}}");
    const before = raw.entries.getUnique("raw").fields.getUnique("title");
    const patched = raw.applyPatch([
      {
        kind: "set_field",
        entryId: before.entryId,
        fieldId: before.id,
        value: "Updated",
      },
    ]);
    return [
      before,
      patched.document.entries.getUnique("raw").fields.getUnique("title"),
    ];
  })();
  await global.gc({ execution: "async" });
  assert.equal(
    document.render([new Citation("id", "a")]).get("id").text,
    "(Doe, 2024)",
  );
  assert.equal(before.value, "Original");
  assert.equal(after.value, "Updated");
});
