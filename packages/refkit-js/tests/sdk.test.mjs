import assert from "node:assert/strict";
import { test } from "node:test";
import * as rk from "refkit-js";

test("duplicate review compiles chosen fields into expression-preserving patches", () => {
  const source =
    "@string{press={Press}}@book{a,title={First},doi={10.1234/work}}@book{b,title={Second},doi={10.1234/work},publisher=press,url={https://example.org/a%2Fb}}@misc{child,crossref={b}}";
  const original = rk.BibDocument.parse(source);
  const report = original.findDuplicates({ rules: ["doi"] });
  assert.deepEqual(
    report.groups[0].members.map((member) => member.key),
    ["a", "b"],
  );
  assert.equal(original.planMerge({ entries: [0, 1], retain: 0 }).patch, null);
  const plan = original.planMerge({
    entries: [0, 1],
    retain: 0,
    fields: [{ kind: "take", name: "title", entryId: 1, fieldId: 0 }],
  });
  const updated = original.applyPatch(plan.patch).document;
  assert.equal(updated.resolve()[0].fields.publisher, "Press");
  assert.equal(updated.resolve()[0].fields.url, "https://example.org/a%2Fb");
  assert.equal(updated.resolve()[1].fields.crossref, "a");
  assert.equal(original.toBibtex(), source);
  assert.throws(
    () => original.planMerge({ entries: [0, 0], retain: 0 }),
    (error) =>
      error instanceof rk.MergeError && error.code === "invalid_selection",
  );
});

test("atomic structural patches return new snapshots and complete mappings", () => {
  const source = "% é\n@book{a,title={Old},note={Remove}}\n@misc{b,title={B}}";
  const original = rk.BibDocument.parse(source);
  const entry = original.entries.getUnique("a");
  const field = entry.fields.getUnique("title");
  const result = original.applyPatch([
    {
      kind: "set_field",
      entryId: entry.id,
      fieldId: field.id,
      value: "Updated",
    },
    {
      kind: "remove_field",
      entryId: entry.id,
      fieldId: entry.fields.getUnique("note").id,
    },
    { kind: "add_field", entryId: entry.id, name: "doi", value: "10.1234/a" },
    { kind: "rename_entry", entryId: entry.id, key: "renamed" },
    { kind: "set_entry_type", entryId: entry.id, entryType: "article" },
    { kind: "remove_entry", entryId: original.entries.getUnique("b").id },
    { kind: "add_entry", before: entry.id, key: "new", entryType: "book" },
  ]);
  assert.equal(original.toBibtex(), source);
  assert.equal(field.value, "Old");
  assert.deepEqual(result.document.entries.occurrenceKeys(), [
    "new",
    "renamed",
  ]);
  assert.equal(result.entries[0].after.id, 1);
  assert.equal(result.entries[0].fields[1].after, null);
  assert.equal(result.entries[1].after, null);
  const operation = {
    kind: "set_field",
    entryId: field.entryId,
    fieldId: field.id,
    value: "New",
  };
  assert.throws(
    () => original.applyPatch([operation, operation]),
    (error) =>
      error instanceof rk.PatchError &&
      error.code === "overlap" &&
      error.operation === 1,
  );
  assert.equal(original.toBibtex(), source);
});

test("validation reports preserve source and detached record state", () => {
  const source = "% é\n@article{a,title={A},doi={bad},crossref={missing}}";
  const raw = rk.BibDocument.parse(source);
  const report = raw.validate();
  assert.equal(report.profile, "biblatex");
  assert.equal(report.valid, false);
  const invalid = report.issues.find(
    (issue) => issue.code === "invalid_identifier",
  );
  assert.equal(invalid.target.entryId, 0);
  assert.equal(invalid.target.fieldId, 1);
  assert.ok(
    new TextDecoder()
      .decode(new TextEncoder().encode(source).slice(...invalid.target.span))
      .includes("bad"),
  );
  assert.equal(raw.toBibtex(), source);
  assert.throws(
    () => rk.BibDocument.parse("@misc{a,title=undefined}").validate(),
    rk.ParseError,
  );
  const library = rk.Library.fromRecords([
    { key: "a", entryType: "Misc", identifiers: { isbn: "invalid" } },
  ]);
  const records = library.validate();
  assert.equal(records.issues[0].target.path, "identifiers.isbn");
  assert.equal(records.issues[0].target.span, null);
  records.issues.length = 0;
  assert.equal(library.validate().issues.length, 1);
});

test("citation purposes and style catalog reach the renderer", () => {
  const catalog = rk.Style.list();
  assert.deepEqual(
    catalog.map((style) => style.name),
    catalog.map((style) => style.name).sort(),
  );
  const apa = catalog.find((style) =>
    [style.name, ...style.aliases].includes("apa"),
  );
  assert.equal(apa.cslId, "http://www.zotero.org/styles/apa");
  const library = rk.Library.parseBibtex(
    "@book{a,author={Doe, Jane},title={A Book},year={2024}}",
  );
  const document = new rk.Document(library, rk.Style.load(apa.name), {
    locale: "en-US",
  });
  const item = new rk.Cite("a", { purpose: "prose" });
  assert.equal(item.purpose, "prose");
  assert.equal(
    document.render([new rk.Citation("intro", item)]).get("intro").text,
    "Doe (2024)",
  );
  assert.throws(
    () => new rk.Cite("a", { purpose: "unknown" }),
    /unknown citation purpose/,
  );
});

const source =
  "@book{doe, author={Doe, Jane}, title={A Book}, year={2024}, publisher={Press}}\n@book{roe, author={Roe, John}, title={Other}, year={2023}}";

test("codecs return renderable libraries and explicit loss reports", () => {
  const source =
    '[{"id":"book","type":"book","title":"A Book","author":[{"family":"Doe","given":"Jane"}],"issued":{"date-parts":[[2024]]},"publisher":"Press"}]';
  const decoded = rk.decode(source, { format: "csl-json", loss: "error" });
  for (const format of ["biblatex", "hayagriva", "csl-json"]) {
    const encoded = rk.encode(decoded.library, { format, loss: "error" });
    assert.equal(encoded.format, format);
    assert.equal(
      encoded.issues.some((issue) => issue.lossy),
      false,
    );
    const restored = rk.decode(encoded.text, { format, loss: "error" }).library;
    assert.equal(rk.cite(restored, "book").text, "(Doe, 2024)");
  }
  const range = rk.decode(
    '[{"id":"a","type":"book","issued":{"date-parts":[[2020],[2024]]}}]',
    { format: "csl-json" },
  ).library;
  const snapshot = range.toJson();
  assert.throws(
    () => rk.encode(range, { format: "hayagriva", loss: "error" }),
    (error) =>
      error instanceof rk.ConversionError &&
      error.issues.some(
        (issue) => issue.path.startsWith("date") && issue.lossy,
      ),
  );
  assert.equal(range.toJson(), snapshot);
  const result = rk.convert("@book{a,title=missing}", {
    sourceFormat: "biblatex",
    targetFormat: "hayagriva",
    recovery: "report",
  });
  assert.equal(result.diagnostics[0].code, "unknown_abbreviation");
  assert.equal(result.sourceFormat, "biblatex");
  assert.equal(result.targetFormat, "hayagriva");
});

test("structured records retain complete values through canonical JSON", () => {
  const input = {
    key: "council",
    entryType: "Book",
    title: { chunks: [{ kind: "normal", text: "Annual Report" }] },
    authors: [{ kind: "organization", name: "Research Council" }],
    date: { value: { kind: "point", date: { year: 2024 } } },
    extensions: {
      app: { entry_type: "untouched", nested: [true, null, 1.25] },
    },
  };
  const library = rk.Library.fromRecords([input]);
  const snapshot = library.toJson();
  const restored = rk.Library.fromJson(snapshot);
  assert.deepEqual(restored.toRecords(), library.toRecords());
  assert.equal(rk.cite(restored, "council").text, "(Research Council, 2024)");
  assert.equal(rk.Library.fromRecords(restored.toRecords()).toJson(), snapshot);
  library.toRecords()[0].title.chunks[0].text = "Changed";
  assert.equal(library.toJson(), snapshot);
  assert.throws(
    () => rk.Library.fromRecords([input, input]),
    /duplicate entry key/,
  );
  assert.throws(
    () => rk.Library.fromRecords([{ key: "x", entryType: "Book", typo: true }]),
    /unknown field/,
  );
  assert.throws(
    () => rk.Library.fromJson('{"schema_version":2,"records":[]}'),
    /schema version 1/,
  );
  assert.throws(
    () =>
      rk.Library.fromRecords([
        { ...input, extensions: { app: { score: Infinity } } },
      ]),
    /finite/,
  );
  assert.throws(
    () =>
      rk.Library.fromRecords([
        { ...input, extensions: { app: { score: () => 1 } } },
      ]),
    TypeError,
  );
});

test("record snapshots retain supported deep container chains", () => {
  let record = { key: "a", entryType: "Book" };
  for (let depth = 0; depth < 64; depth++)
    record = { key: "a", entryType: "Book", parents: [record] };
  const library = rk.Library.fromRecords([record]);
  assert.equal(
    rk.Library.fromJson(library.toJson()).toJson(),
    library.toJson(),
  );
  assert.equal(
    rk.Library.fromRecords(library.toRecords()).toJson(),
    library.toJson(),
  );
});

test("supplied style parent preserves child identity and locale precedence", () => {
  const child = `<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" default-locale="de-DE">
    <info><title>Child</title><id>https://example.com/child</id>
    <link rel="independent-parent" href="https://example.com/parent"/></info></style>`;
  const parent = `<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
    <info><title>Parent</title><id>https://example.com/parent</id></info>
    <locale xml:lang="de-DE"><terms><term name="page">Seite</term></terms></locale>
    <locale xml:lang="en-US"><terms><term name="page">page</term></terms></locale>
    <citation><layout><text term="page"/></layout></citation></style>`;
  const style = rk.Style.fromXml(child, { parentXml: parent });
  assert.equal(style.title, "Child");
  assert.equal(style.cslId, "https://example.com/child");
  const library = rk.Library.parseBibtex(source);
  const requests = [new rk.Citation("term", "doe")];
  assert.equal(
    new rk.Document(library, style).render(requests).get("term").text,
    "Seite",
  );
  assert.equal(
    new rk.Document(library, style, { locale: "en-US" })
      .render(requests)
      .get("term").text,
    "page",
  );
  assert.throws(() => rk.Style.fromXml(child), /supply parent XML/);
  assert.throws(
    () =>
      rk.Style.fromXml(child, {
        parentXml: parent.replace(
          "https://example.com/parent",
          "https://example.com/wrong",
        ),
      }),
    /expected/,
  );
});

test("Node import initializes the portable API", () => {
  const library = rk.Library.parseBibtex(source);
  assert.equal(rk.getBuildInfo().version, rk.version);
  assert.equal(library.size, 2);
  assert.deepEqual(library.keys(), ["doe", "roe"]);
  assert.equal(library.get("missing"), null);
  assert.equal(library.has("doe"), true);
  assert.equal(library.has("missing"), false);
  assert.equal(library.isEmpty(), false);
  assert.deepEqual(
    [...library].map((entry) => entry.key),
    ["doe", "roe"],
  );
});

test("projections retain selected order and null fields", () => {
  const library = rk.Library.parseBibtex(source);
  assert.deepEqual(library.project(), [
    { key: "doe", title: "A Book", doi: null, volume: null },
    { key: "roe", title: "Other", doi: null, volume: null },
  ]);
  assert.deepEqual(
    library.project(["key", "entryType"], { keys: ["roe", "doe", "roe"] }),
    [
      { key: "roe", entryType: "Book" },
      { key: "doe", entryType: "Book" },
      { key: "roe", entryType: "Book" },
    ],
  );
  assert.throws(() => library.getMany(["missing"]), rk.MissingReferenceError);
  assert.throws(() => library.project(["unknown"]), RangeError);
});

test("returned entries cannot mutate the library", () => {
  const library = rk.Library.parseBibtex(source);
  const record = library.get("doe");
  record.title.chunks[0].text = "Changed";
  assert.equal(library.get("doe").title.chunks[0].text, "A Book");
});

test("bulk lookup preserves generator order and detached duplicate records", () => {
  const library = rk.Library.parseBibtex(source);
  function* keys() {
    yield "roe";
    yield "doe";
    yield "roe";
  }
  const records = library.getMany(keys());
  assert.deepEqual(
    records.map((record) => record.key),
    ["roe", "doe", "roe"],
  );
  records[0].title.chunks[0].text = "Changed";
  assert.equal(records[2].title.chunks[0].text, "Other");
  assert.equal(library.get("roe").title.chunks[0].text, "Other");
  assert.deepEqual(library.getMany([]), []);
  assert.throws(
    () => library.getMany(["doe", "missing"]),
    rk.MissingReferenceError,
  );
});

test("raw metadata returns detached source-ordered values", () => {
  const input =
    "% Café\n@string{press={First}}\n@string{press={Last}}\n" +
    '@preamble{"Prefix"}\n@book{a,title={Book}}\n@book{broken,title={unclosed';
  const document = rk.BibDocument.parse(input);
  assert.deepEqual(document.diagnostics, []);
  assert.equal(document.preamble, "Prefix");
  assert.deepEqual(document.strings, { press: "Last" });
  assert.equal(
    document.comments.some((comment) => comment.includes("Café")),
    true,
  );
  assert.equal(document.failedBlocks.length, 1);
  for (const property of ["comments", "strings", "failedBlocks", "blocks"]) {
    const expected = document[property];
    const detached = document[property];
    if (Array.isArray(detached)) {
      if (property === "blocks" || property === "failedBlocks") {
        detached[0].span[0] = -1;
      }
      detached.length = 0;
    } else {
      detached.press = "Changed";
    }
    assert.deepEqual(document[property], expected);
  }
  assert.equal(document.toBibtex(), input);
});

test("parse errors and recovery expose UTF-8 diagnostic spans", () => {
  const input = "@book{good,title={Café}}\n@book{bad,title=未定}";
  assert.throws(
    () => rk.Library.parseBibtex(input),
    (error) => error instanceof rk.ParseError && error.diagnostics.length > 0,
  );
  const library = rk.Library.parseBibtex(input, { recovery: "report" });
  const diagnostic = library.diagnostics.find(
    (item) => item.code === "unknown_abbreviation",
  );
  const bytes = new TextEncoder().encode(input);
  assert.equal(
    new TextDecoder().decode(bytes.slice(...diagnostic.span)),
    "未定",
  );
  assert.equal(library.get("bad").title.chunks[0].text, "未定");
  assert.throws(
    () => rk.Library.parseBibtex(source, { recovery: "typo" }),
    RangeError,
  );
});

test("document rendering shares citation state within a call and resets between calls", () => {
  const library = rk.Library.parseBibtex(source);
  const style = rk.Style.load("ieee");
  const document = new rk.Document(library, style);
  const result = document.render([
    new rk.Citation("first", "roe"),
    new rk.Citation("second", "doe"),
  ]);
  assert.deepEqual(result.citationOrder, ["first", "second"]);
  assert.equal(result.get("first").text, "[1]");
  assert.equal(result.get("second").text, "[2]");
  assert.equal(
    document.render([new rk.Citation("again", "doe")]).get("again").text,
    "[1]",
  );
});

test("rendering rejects duplicate IDs and missing references", () => {
  const document = new rk.Document(
    rk.Library.parseBibtex(source),
    rk.Style.load("apa"),
  );
  assert.throws(
    () =>
      document.render([
        new rk.Citation("same", "doe"),
        new rk.Citation("same", "roe"),
      ]),
    RangeError,
  );
  assert.throws(
    () => document.render([new rk.Citation("missing", "absent")]),
    rk.MissingReferenceError,
  );
});

test("citation helpers accept prepared styles, locales, and locators", () => {
  const library = rk.Library.parseBibtex(source);
  const options = {
    style: rk.Style.load("apa"),
    locale: rk.Locale.load("en-US"),
  };
  const rendered = rk.cite(
    library,
    new rk.Cite("doe", { locator: "12", label: "page" }),
    options,
  );
  assert.equal(rendered.text, "(Doe, 2024, p. 12)");
  assert.deepEqual(
    rk.fullBibliography(library, options).tree.map((entry) => entry.key),
    ["doe", "roe"],
  );
});

test("citation IDs use own keys including object prototype names", () => {
  const library = rk.Library.parseBibtex(source);
  const style = rk.Style.load("apa");
  const document = new rk.Document(library, style);
  const result = document.render([
    new rk.Citation("__proto__", "doe"),
    new rk.Citation("constructor", "roe"),
  ]);
  assert.equal(result.get("__proto__").text, "(Doe, 2024)");
  assert.equal(result.get("constructor").text, "(Roe, 2023)");
  assert.throws(() => result.get("toString"), rk.MissingReferenceError);
});

test("citation input validation preserves integer and iterable boundaries", () => {
  for (const noteNumber of [0, 1.5, NaN, 4294967296])
    assert.throws(
      () => new rk.Citation("x", "doe", { noteNumber }),
      RangeError,
    );
  assert.equal(
    new rk.Citation("x", "doe", { noteNumber: 4294967295 }).noteNumber,
    4294967295,
  );
  assert.throws(() => new rk.CitationGroup([]), RangeError);
  assert.throws(() => new rk.CitationGroup("doe"), TypeError);
  const group = new rk.CitationGroup(["doe", new rk.Cite("roe")]);
  assert.equal(group.size, 2);
  assert.deepEqual(
    [...group].map((item) => item.key),
    ["doe", "roe"],
  );
});

test("raw edits preserve duplicate occurrence identity and original spans", () => {
  const input =
    "% Café\n@book{same, TITLE={First}, title={Second}}\n@book{same,title={Third}}";
  const document = rk.BibDocument.parse(input);
  assert.equal(document.entries.size, 2);
  assert.equal(document.entries.has("same"), true);
  assert.equal(document.entries.has("missing"), false);
  assert.deepEqual(document.entries.uniqueKeys(), ["same"]);
  assert.deepEqual(document.entries.occurrenceKeys(), ["same", "same"]);
  assert.throws(() => document.entries.getUnique("same"), rk.RefkitError);
  const [first, second] = document.entries.getAll("same");
  assert.equal(first.fields.size, 2);
  assert.equal(first.fields.has("TITLE"), true);
  assert.equal(first.fields.has("missing"), false);
  assert.throws(() => first.fields.getUnique("title"), rk.RefkitError);
  const field = first.fields.getAll("TITLE")[1];
  const span = field.span;
  const patched = document.applyPatch([
    {
      kind: "set_field",
      entryId: field.entryId,
      fieldId: field.id,
      value: "Changed",
    },
  ]);
  assert.equal(field.value, "Second");
  assert.deepEqual(field.span, span);
  field.span[0] = -1;
  assert.deepEqual(field.span, span);
  assert.match(
    patched.document.toBibtex(),
    /TITLE=\{First\}, title=\{Changed\}/,
  );
  assert.equal(document.toBibtex(), input);
  assert.equal(second.fields.getUnique("title").value, "Third");
  assert.equal(document.entries.getUnique("missing"), null);
  assert.equal(first.fields.getUnique("missing"), null);
  assert.throws(() => {
    document.applyPatch([
      {
        kind: "set_field",
        entryId: field.entryId,
        fieldId: field.id,
        value: "{unclosed",
      },
    ]);
  }, rk.PatchError);
  assert.equal(field.value, "Second");
});

test("document tidy formats current edits while preserving editable source", () => {
  const document = rk.BibDocument.parse(
    "@book{draft,title={Draft},note={Internal}}",
  );
  const field = document.entries.getUnique("draft").fields.getUnique("title");
  const updated = document.applyPatch([
    {
      kind: "set_field",
      entryId: field.entryId,
      fieldId: field.id,
      value: "Published",
    },
  ]).document;
  const edited = updated.toBibtex();
  const result = rk.BibDocument.parse(
    updated.tidy({ options: { omit: ["note"] } }).bibtex,
  );
  const entry = result.entries.getUnique("draft");
  assert.deepEqual(entry.fields.uniqueKeys(), ["title"]);
  assert.equal(entry.fields.getUnique("title").value, "Published");
  assert.equal(updated.toBibtex(), edited);
  assert.equal(field.value, "Draft");
});

test("raw resolution exposes expanded fields and current edits as detached records", () => {
  const document = rk.BibDocument.parse(String.raw`
@string{host = "https://example.test/"}
@misc{entry,
  title = {A {Protected} \LaTeX{} Title},
  url = host # {paper},
  custom = {Keep Me},
  month = jan
}`);
  const before = document.toBibtex();
  const [record] = document.resolve();
  assert.deepEqual(record, {
    key: "entry",
    entryType: "misc",
    fields: {
      title: String.raw`A {Protected} \LaTeX{} Title`,
      url: "https://example.test/paper",
      custom: "Keep Me",
      month: "January",
    },
  });
  assert.equal(document.toBibtex(), before);
  record.fields.url = "Changed outside document";
  assert.equal(document.resolve()[0].fields.url, "https://example.test/paper");
  const field = document.entries.getUnique("entry").fields.getUnique("url");
  const updated = document.applyPatch([
    {
      kind: "set_field",
      entryId: field.entryId,
      fieldId: field.id,
      value: "https://example.test/revised",
    },
  ]).document;
  assert.equal(updated.resolve()[0].fields.url, "https://example.test/revised");
});

test("raw resolution errors retain structured diagnostics across WebAssembly", () => {
  const source = "@misc{entry,title=未定}";
  assert.throws(
    () => rk.BibDocument.parse(source).resolve(),
    (error) => {
      assert.ok(error instanceof rk.ParseError);
      const diagnostic = error.diagnostics.find(
        (item) => item.code === "unknown_abbreviation",
      );
      assert.ok(diagnostic);
      assert.equal(diagnostic.entry, "entry");
      assert.equal(diagnostic.field, "title");
      assert.equal(
        new TextDecoder().decode(
          new TextEncoder().encode(source).slice(...diagnostic.span),
        ),
        "未定",
      );
      return true;
    },
  );
});

test("tidy rejects invalid option types and values", () => {
  for (const options of [{ space: -1 }, { space: 1.5 }, { sort: "key" }]) {
    assert.throws(
      () => rk.tidyBibtex("@book{a,title={A}}", { options }),
      TypeError,
    );
  }
  for (const options of [
    { mystery: true },
    { duplicates: ["typo"] },
    { merge: "typo" },
  ]) {
    assert.throws(
      () => rk.tidyBibtex("@book{a,title={A}}", { options }),
      RangeError,
    );
  }
});

test("tidy syntax errors identify the source position", () => {
  assert.throws(
    () => rk.tidyBibtex("@book{"),
    (error) => {
      assert.ok(error instanceof rk.TidySyntaxError);
      assert.deepEqual(
        [error.line, error.column, error.byte, error.character],
        [1, 1, 0, "@"],
      );
      return true;
    },
  );
});

test("JavaScript option values retain their types across WebAssembly serialization", () => {
  for (const options of [{ align: NaN }, { omit: [undefined] }]) {
    assert.throws(
      () => rk.tidyBibtex("@book{a,title={A}}", { options }),
      TypeError,
    );
  }
  assert.throws(
    () => rk.Library.parseBibtex(source, { recover: "report" }),
    TypeError,
  );
});
