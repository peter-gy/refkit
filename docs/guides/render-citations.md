---
description: Render named citations and cited or full bibliographies with bundled or explicit CSL styles in Python or TypeScript.
---

# Render Citations

`Document` renders an ordered sequence of citations against a `Library`, a [Citation Style Language](https://citationstyles.org/) (CSL) style, and a locale. CSL defines citation and bibliography formatting, while the locale supplies language-specific terms.

## Render named citations

::: code-group

```python [Python]
import refkit as rk

source = """
@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}
@book{roe2022, author={Roe, Richard}, title={Citation Practice}, year={2022}}
"""
library = rk.Library.parse_bibtex(source)
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
result = document.render(
    [
        rk.Citation("opening", "doe2024"),
        rk.Citation(
            "details",
            rk.CitationGroup(
                [
                    rk.Cite("doe2024", locator="12", label="page"),
                    "roe2022",
                ]
            ),
        ),
    ]
)
print(result["opening"].text)
print(result["details"].text)
```

```ts [TypeScript]
import * as rk from "refkit-js";

const source = `
@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}
@book{roe2022, author={Roe, Richard}, title={Citation Practice}, year={2022}}
`;
const library = rk.Library.parseBibtex(source);
const document = new rk.Document(library, rk.Style.load("apa"), { locale: "en-US" });
const result = document.render([
  new rk.Citation("opening", "doe2024"),
  new rk.Citation("details", new rk.CitationGroup([
    new rk.Cite("doe2024", { locator: "12", label: "page" }),
    "roe2022",
  ])),
]);
console.log(result.get("opening").text);
console.log(result.get("details").text);
```

:::

```text
(Doe, 2024)
(Doe, 2024, p. 12; Roe, 2022)
```

TypeScript examples run in Node.js. In a browser, [initialize RefKit](/guides/browser#initialize-the-module) before creating the library. The remaining examples continue with `library`, `document`, and `result`.

Citation IDs identify rendered results and must be unique inside one call. Pass the complete ordered sequence to preserve numbering, subsequent-citation rules, disambiguation, and bibliography contents. Each render call starts fresh citation state.

## Add a locator

`Cite` accepts a citation key plus a `locator` and `label`:

::: code-group

```python [Python]
page = rk.Cite("doe2024", locator="12", label="page")
```

```ts [TypeScript]
const page = new rk.Cite("doe2024", { locator: "12", label: "page" });
```

:::

The style controls the visible form. An unknown locator label raises `ValueError` in Python or `RangeError` in TypeScript during rendering.

## Cite a numbered note

Give a citation its document note number when the style uses note distance or first-reference note numbers:

::: code-group

```python [Python]
note = rk.Citation("detail-note", page, note_number=8)
```

```ts [TypeScript]
const note = new rk.Citation("detail-note", page, { noteNumber: 8 });
```

:::

The note number belongs to the citation occurrence. Pass the complete ordered sequence to `Document.render` so repeated citations can use earlier notes.

## Render a bibliography

The render result includes the cited bibliography. To request a bibliography directly, choose cited entries or the complete library:

::: code-group

```python [Python]
print(result.bibliography.text)
cited = document.cited_bibliography([rk.Citation("opening", "doe2024")])
complete = document.full_bibliography()
```

```ts [TypeScript]
console.log(result.bibliography.text);
const cited = document.citedBibliography([new rk.Citation("opening", "doe2024")]);
const complete = document.fullBibliography();
```

:::

`cited` contains entries referenced by the supplied citations. `complete` contains every entry in the library. Both results expose `.text`, `.html`, `.tree`, and `.layout`.

## Render one citation

The convenience helper renders one citation with a named or prepared style. Python's helper reads a bibliography path. TypeScript's helper accepts a prepared `Library`:

::: code-group

```python [Python]
citation = rk.cite("references.bib", "doe2024", style="ieee")
print(citation.text)
```

```ts [TypeScript]
const citation = rk.cite(library, "doe2024", { style: "ieee" });
console.log(citation.text);
```

:::

Pass a `Cite` or `CitationGroup` for richer input. Use `Document` when several citations share document order.

## Load an explicit style

`Style` accepts an independent CSL style as XML text:

::: code-group

```python [Python]
csl_xml = """<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info>
    <title>Title citations</title>
    <id>https://example.org/styles/title-citations</id>
    <updated>2026-01-01T00:00:00+00:00</updated>
  </info>
  <citation><layout><text variable="title"/></layout></citation>
</style>"""
custom_style = rk.Style.from_xml(csl_xml)
custom_document = rk.Document(library, custom_style, locale="en-US")
print(custom_document.render([rk.Citation("title", "doe2024")])["title"].text)
```

```ts [TypeScript]
const cslXml = `<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info>
    <title>Title citations</title>
    <id>https://example.org/styles/title-citations</id>
    <updated>2026-01-01T00:00:00+00:00</updated>
  </info>
  <citation><layout><text variable="title"/></layout></citation>
</style>`;
const customStyle = rk.Style.fromXml(cslXml);
const customDocument = new rk.Document(library, customStyle, { locale: "en-US" });
console.log(customDocument.render([new rk.Citation("title", "doe2024")]).get("title").text);
```

:::

This style renders `Fast Citations`. For a UTF-8 `.csl` file, use `Style.from_path(path)` in Python or `await readStyle(path)` from `refkit-js/node` in Node.js.

Custom XML is limited to 2 MiB, 100,000 XML nodes, 64 nested elements, and 256 attributes per element, including namespace declarations. Expanded rendering is limited to 100,000 elements and 64 levels of combined element nesting and macro calls. Missing, duplicate, or cyclic macros and exceeded limits raise `ValueError` / `RangeError`. A dependent style that requires parent resolution also raises an error. Use `Style.load(name)` for a style in the [bundled archive](/concepts/citation-rendering).

## Render safely for the web

`Rendered.html` emits CSL markup and escapes bibliography data. Link nodes allow safe URL schemes. A URL with an unsafe scheme remains visible as text.

Use `Rendered.tree` when an application needs to control element creation and styling. [Render Structured Output](/guides/render-output) shows a complete custom text consumer and explains source identity, formatting, and bibliography layout.
