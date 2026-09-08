import { Citation, Document, Library, Style } from "refkit-js";

const library = Library.parseBibtex(`
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  journal = {Journal of Citation Tests},
  year = {2024}
}
`);
const style = Style.load("apa");
const document = new Document(library, style, { locale: "en-US" });
const result = document.render([new Citation("intro", "doe2024")]);
console.log(result.get("intro").text);
console.log(result.bibliography.text);
