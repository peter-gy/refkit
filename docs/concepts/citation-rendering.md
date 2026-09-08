---
description: Build an ordered citation document and inspect its citations, bibliography, HTML, text, or render tree.
---

# Citation Rendering

Citation rendering transforms normalized entries through a [Citation Style Language (CSL)](https://citationstyles.org/) style, a format that describes citation and bibliography rules. One render operation owns the full ordered citation list and the bibliography derived from it.

## Build a citation document

::: code-group

```python [Python]
import refkit as rk

library = rk.Library.parse_bibtex("""
@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}
@book{roe2022, author={Roe, Richard}, title={Batch References}, year={2022}}
""")
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
rendered = document.render(
    [
        rk.Citation("intro", "doe2024"),
        rk.Citation(
            "detail",
            rk.CitationGroup(
                [
                    rk.Cite("doe2024", locator="12", label="page"),
                    "roe2022",
                ]
            ),
        ),
    ]
)
print(rendered["intro"].text)
print(rendered["detail"].text)
```

```ts [TypeScript]
import * as rk from "refkit-js";

const library = rk.Library.parseBibtex(`
@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}
@book{roe2022, author={Roe, Richard}, title={Batch References}, year={2022}}
`);
const document = new rk.Document(library, rk.Style.load("apa"), { locale: "en-US" });
const rendered = document.render([
  new rk.Citation("intro", "doe2024"),
  new rk.Citation("detail", new rk.CitationGroup([
    new rk.Cite("doe2024", { locator: "12", label: "page" }),
    "roe2022",
  ])),
]);
console.log(rendered.get("intro").text);
console.log(rendered.get("detail").text);
```

:::

Both produce `(Doe, 2024)` for `intro` and `(Doe, 2024, p. 12; Roe, 2022)` for `detail`. TypeScript examples run in Node.js. For browsers, complete [browser initialization](/guides/browser#initialize-the-module) first.

The objects describe the document at distinct levels:

| Object | Owns |
| --- | --- |
| `Cite` | One citation key and optional locator such as a page number. |
| `CitationGroup` | One or more items rendered as one citation cluster. |
| `Citation` | A group, a unique result ID, and an optional document note number. |
| `Document` | A library, style, and locale prepared for rendering. |
| `RenderedDocument` | Named citation outputs and the cited bibliography. |

## Keep the full order together

Each render call creates fresh citation state. Pass the complete ordered citation list when citations depend on earlier disambiguation or numbering. The result preserves the input IDs in citation order and supports lookup by ID, as `intro` and `detail` demonstrate.

## Choose a bibliography boundary

The `rendered` result includes the entries cited by that call. The document can also produce a bibliography of every entry in the library:

::: code-group

```python [Python]
print(rendered.bibliography.text)
print(document.full_bibliography().text)
```

```ts [TypeScript]
console.log(rendered.bibliography.text);
console.log(document.fullBibliography().text);
```

:::

Both bibliographies contain Doe and Roe because this citation list cites both entries. A document's cited-bibliography method accepts a citation list and returns its bibliography directly. See the [Python](/reference/python#rendering) and [TypeScript](/reference/javascript#rendering) references for the exact methods.

## Choose an output representation

Every `Rendered` value exposes `text` for plain text, `html` for escaped HTML, and `tree` for structured nodes and bibliography records. The tree retains formatting, links, and source identity for a host renderer. Bibliography outputs also provide `layout`, including hanging indent, label alignment, and line and entry spacing. Read [Data Shapes](/reference/data-shapes) for the node contract.

## Load styles and locales

`Style.load(name)` resolves a bundled style such as `apa`, `ieee`, or `chicago-author-date`. To prepare a custom style, parse CSL XML text or read a local style file through the host's file API. The [Python](/reference/python#styles-and-citations) and [TypeScript](/reference/javascript#styles-and-citation-inputs) references describe both paths.

`Locale.load(code)` validates a bundled locale, which supplies translated terms and date conventions. A document accepts a locale code directly, as `en-US` demonstrates. Pass a `Locale` object when validation should happen before document construction.

Continue with [Render Citations](/guides/render-citations) for complete rendering workflows.
