import {
  Citation,
  Cite,
  Document,
  Library,
  Style,
  tidyBibtex,
  type Entry,
  type RenderedNode,
  type TidyOptions,
} from "refkit-js";
import { init } from "refkit-js/browser";
import { readLibrary } from "refkit-js/node";
const library: Library = Library.parseBibtex("@book{a,title={A}}");
const entry: Entry | null = library.get("a");
const options: TidyOptions = {
  align: true,
  sort: ["key"],
  generateKeys: "[auth][year]",
};
const result = tidyBibtex("@book{a,title={A}}", { options });
const style = Style.load("apa");
const document = new Document(library, style, { locale: "en-US" });
const rendered = document.render([
  new Citation("a", new Cite("a", { locator: "1", label: "page" })),
]);
const text: string = rendered.get("a").text;
function inspect(node: RenderedNode): string {
  return node.kind === "Text" ? node.text : node.kind;
}
void [entry, result, text, inspect, init, readLibrary];
// @ts-expect-error recovery is a finite vocabulary
Library.parseBibtex("", { recovery: "ignore" });
// @ts-expect-error formatting options preserve boolean values
tidyBibtex("", { options: { curly: "yes" } });
// @ts-expect-error projection fields are a finite vocabulary
library.project(["unknown"]);
