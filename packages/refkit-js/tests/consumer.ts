import {
  BibDocument,
  Citation,
  Cite,
  Document,
  Library,
  Style,
  tidyBibtex,
  decode,
  encode,
  convert,
  type DecodeReport,
  type EncodeReport,
  type ConversionReport,
  type Entry,
  type CitePurpose,
  type StyleMetadata,
  type RenderedNode,
  type ResolvedBibEntry,
  type TidyOptions,
  type ValidationReport,
  type BibPatch,
  type BibPatchResult,
  type DuplicateReport,
  type MergePlan,
} from "refkit-js";
import { init } from "refkit-js/browser";
import { readLibrary } from "refkit-js/node";
const library: Library = Library.parseBibtex("@book{a,title={A}}");
const validation: ValidationReport = library.validate();
const raw = BibDocument.parse("@book{a,title={A}}");
const rawField = raw.entries.getUnique("a")!.fields.getUnique("title")!;
const patch: BibPatch = [
  {
    kind: "set_field",
    entryId: rawField.entryId,
    fieldId: rawField.id,
    value: "B",
  },
];
const patched: BibPatchResult = raw.applyPatch(patch);
void patched.entries[0]?.after?.span;
const duplicateDocument = BibDocument.parse(
  "@book{a,title={A}}@book{b,title={A}}",
);
const duplicateReport: DuplicateReport = duplicateDocument.findDuplicates();
const mergePlan: MergePlan = duplicateDocument.planMerge({
  entries: [0, 1],
  retain: 0,
});
if (mergePlan.patch) duplicateDocument.applyPatch(mergePlan.patch);
void duplicateReport.groups[0]?.conflicts[0]?.values[0]?.expression;
void validation.issues[0]?.target.entryId;
void BibDocument.parse("@book{a,title={A}}").validate().valid;
const decoded: DecodeReport = decode('[{"id":"a","type":"book"}]', {
  format: "csl-json",
});
const encoded: EncodeReport = encode(decoded.library, { format: "hayagriva" });
const converted: ConversionReport = convert(encoded.text, {
  sourceFormat: "hayagriva",
  targetFormat: "biblatex",
});
void converted;
const entry: Entry | null = library.get("a");
const structured = Library.fromRecords([
  {
    key: "book",
    entryType: "Book",
    title: { chunks: [{ kind: "normal", text: "A Book" }] },
    authors: [{ kind: "person", family: "Doe", given: "Jane" }],
  },
]);
const records: Entry[] = structured.toRecords();
const snapshot: string = structured.toJson();
void [records, Library.fromJson(snapshot)];
const resolved: readonly ResolvedBibEntry[] = BibDocument.parse(
  '@string{prefix="A"}\n@book{a,title=prefix # { Book}}',
).resolve();
const title: string | undefined = resolved[0]?.fields.title;
const options: TidyOptions = {
  align: true,
  sort: ["key"],
  generateKeys: "[auth][year]",
};
const result = tidyBibtex("@book{a,title={A}}", { options });
const style = Style.load("apa");
const catalog: StyleMetadata[] = Style.list();
const purpose: CitePurpose = new Cite("a", { purpose: "prose" }).purpose;
const document = new Document(library, style, { locale: "en-US" });
const rendered = document.render([
  new Citation("a", new Cite("a", { locator: "1", label: "page" })),
]);
const text: string = rendered.get("a").text;
function inspect(node: RenderedNode): string {
  return node.kind === "Text" ? node.text : node.kind;
}
void [
  entry,
  resolved,
  title,
  result,
  text,
  inspect,
  init,
  readLibrary,
  catalog,
  purpose,
];
// @ts-expect-error citation purpose is a finite vocabulary
new Cite("a", { purpose: "unknown" });
// @ts-expect-error resolved fields are read-only
resolved[0]!.fields.title = "Changed";
// @ts-expect-error resolved entries are read-only
resolved.push({ key: "new", entryType: "book", fields: {} });
// @ts-expect-error recovery is a finite vocabulary
Library.parseBibtex("", { recovery: "ignore" });
// @ts-expect-error formatting options preserve boolean values
tidyBibtex("", { options: { curly: "yes" } });
// @ts-expect-error projection fields are a finite vocabulary
library.project(["unknown"]);
