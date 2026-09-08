import assert from "node:assert/strict";
import { test } from "node:test";
import { BibDocument, Citation, Document, Library, Style } from "refkit-js";

test("documents and fields retain their native state across garbage collection", async () => {
  const document = new Document(
    Library.parseBibtex("@book{a,author={Doe, Jane},title={A},year={2024}}"),
    Style.load("apa"),
  );
  const field = BibDocument.parse("@book{raw,title={Original}}")
    .entries.getUnique("raw")
    .fields.getUnique("title");
  await global.gc({ execution: "async" });
  assert.equal(
    document.render([new Citation("id", "a")]).get("id").text,
    "(Doe, 2024)",
  );
  field.value = "Updated";
  await global.gc({ execution: "async" });
  assert.equal(field.value, "Updated");
});
