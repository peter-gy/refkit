---
description: Install RefKit in Python or TypeScript and render the same citation and bibliography.
---

# Get Started

RefKit parses a bibliography and applies a [Citation Style Language](https://citationstyles.org/) (CSL) style to produce citations and a bibliography. Choose a language tab to follow the same workflow in Python or [TypeScript](https://www.typescriptlang.org/), JavaScript with checked types.

## Install

Use Python 3.10 through 3.14 or [Node.js](https://nodejs.org/) 22.19 or newer. The package managers [pip](https://pip.pypa.io/) and [npm](https://docs.npmjs.com/) install the corresponding binding:

::: code-group

```bash [Python]
python -m pip install refkit
```

```bash [TypeScript]
npm install refkit-js
```

:::

Node.js initializes RefKit during import. Browser applications [initialize the WebAssembly module](/guides/browser#initialize-the-module) before using the same objects.

::: details Building Python from source
Pip selects a compatible wheel when available. A source build requires Rust and Git and retrieves pinned revisions of the [Hayagriva bibliography engine](https://github.com/typst/hayagriva) and [Citationberg CSL model](https://github.com/typst/citationberg). The first build needs network access unless those revisions are cached.
:::

## Render a citation

Save the example as `first_citation.py` or `first_citation.ts`:

::: code-group

```python [Python]
import refkit as rk

source = """
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  journal = {Journal of Citation Tests},
  year = {2024}
}
"""
library = rk.Library.parse_bibtex(source)
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
rendered = document.render([rk.Citation("intro", "doe2024")])

print(rendered["intro"].text)
print(rendered.bibliography.text)
```

```ts [TypeScript]
import * as rk from "refkit-js";

const source = `
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  journal = {Journal of Citation Tests},
  year = {2024}
}
`;
const library = rk.Library.parseBibtex(source);
const document = new rk.Document(library, rk.Style.load("apa"), { locale: "en-US" });
const rendered = document.render([new rk.Citation("intro", "doe2024")]);

console.log(rendered.get("intro").text);
console.log(rendered.bibliography.text);
```

:::

Run the file:

::: code-group

```bash [Python]
python first_citation.py
```

```bash [TypeScript]
node first_citation.ts
```

:::

Both produce:

```text
(Doe, 2024)
Doe, J. (2024). Fast Citations. Journal of Citation Tests.
```

`Library` holds normalized references. `Style` selects the citation rules. `Document` combines them with a locale, and each `Citation` names a result. `RenderedDocument` gives you the named citations and the bibliography of entries they cite.

## Choose the next task

| Task | Continue with |
| --- | --- |
| Understand normalized records and editable source | [Choose a bibliography model](/concepts/bibliography-models) |
| Inspect entries, fields, and diagnostics | [Parse bibliographies](/guides/parse-bibliographies) |
| Render groups, locators, and ordered citations | [Render citations](/guides/render-citations) |
| Preserve comments and duplicate occurrences during edits | [Edit raw BibTeX](/guides/edit-bibtex) |
| Canonically format a bibliography | [Format BibTeX](/guides/format-bibtex) |
| Process Python dataframe columns | [Use Polars expressions](/guides/polars) |

[How RefKit works](/concepts/how-refkit-works) connects these tasks through the shared object model.
