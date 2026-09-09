import assert from "node:assert/strict";
import { test } from "node:test";
import * as rk from "refkit-js";

const source =
  "@book{doe, author={Doe, Jane}, title={A Book}, year={2024}, publisher={Press}}\n@book{roe, author={Roe, John}, title={Other}, year={2023}}";

test("Node import initializes the portable API", () => {
  const library = rk.Library.parseBibtex(source);
  assert.equal(rk.getBuildInfo().version, rk.version);
  assert.equal(library.size, 2);
  assert.deepEqual(library.keys(), ["doe", "roe"]);
  assert.equal(library.get("missing"), null);
  assert.equal(library.has("doe"), true);
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
  record.title = "Changed";
  assert.equal(library.get("doe").title, "A Book");
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
  assert.equal(library.get("bad").title, "未定");
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
  assert.deepEqual(document.entries.uniqueKeys(), ["same"]);
  assert.deepEqual(document.entries.occurrenceKeys(), ["same", "same"]);
  assert.throws(() => document.entries.getUnique("same"), rk.RefkitError);
  const [first, second] = document.entries.getAll("same");
  assert.equal(first.fields.size, 2);
  assert.throws(() => first.fields.getUnique("title"), rk.RefkitError);
  const field = first.fields.getAll("TITLE")[1];
  const span = field.span;
  field.value = "Changed";
  assert.equal(field.value, "Changed");
  assert.deepEqual(field.span, span);
  assert.match(document.toBibtex(), /TITLE=\{First\}, title=\{Changed\}/);
  assert.equal(second.fields.getUnique("title").value, "Third");
  assert.equal(document.entries.getUnique("missing"), null);
  assert.equal(first.fields.getUnique("missing"), null);
  assert.throws(() => {
    field.value = "{unclosed";
  }, RangeError);
  assert.equal(field.value, "Changed");
});

test("document tidy formats current edits while preserving editable source", () => {
  const document = rk.BibDocument.parse(
    "@book{draft,title={Draft},note={Internal}}",
  );
  document.entries.getUnique("draft").fields.getUnique("title").value =
    "Published";
  const edited = document.toBibtex();
  const result = rk.BibDocument.parse(
    document.tidy({ options: { omit: ["note"] } }).bibtex,
  );
  const entry = result.entries.getUnique("draft");
  assert.deepEqual(entry.fields.uniqueKeys(), ["title"]);
  assert.equal(entry.fields.getUnique("title").value, "Published");
  assert.equal(document.toBibtex(), edited);
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
  document.entries.getUnique("entry").fields.getUnique("url").value =
    "https://example.test/revised";
  assert.equal(
    document.resolve()[0].fields.url,
    "https://example.test/revised",
  );
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
