---
description: Read rendered nodes in a custom consumer while preserving bibliography labels, layout, and source identity.
---

# Render Structured Output

`Rendered.tree` lets an application consume citation output directly. Each node identifies text, formatting, links, nesting, or bibliography entries. This text consumer handles every node kind:

::: code-group

```python [Python]
import refkit as rk
from refkit.types import BibliographyEntry, RenderedNode


def visible_text(node: RenderedNode | BibliographyEntry) -> str:
    match node["kind"]:
        case "Text" | "Link":
            return node["text"]
        case "Markup":
            return node["value"]
        case "Element":
            return "".join(visible_text(child) for child in node["children"])
        case "bibliography-entry":
            label = visible_text(node["label"]) if node["label"] else ""
            content = "".join(visible_text(child) for child in node["content"])
            return f"{label} {content}".strip()
        case "Transparent":
            return ""
    raise ValueError(f"Unknown node kind: {node['kind']}")


library = rk.Library.parse_bibtex(
    "@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}"
)
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
result = document.render([rk.Citation("intro", "doe2024")])
text = "".join(visible_text(node) for node in result["intro"].tree)
print(text)
assert text == "(Doe, 2024)"
assert result.bibliography.layout is not None
```

```ts [TypeScript]
import * as rk from "refkit-js";
import type { BibliographyEntry, RenderedNode } from "refkit-js";

function visibleText(node: RenderedNode | BibliographyEntry): string {
  switch (node.kind) {
    case "Text":
    case "Link":
      return node.text;
    case "Markup":
      return node.value;
    case "Element":
      return node.children.map(visibleText).join("");
    case "bibliography-entry": {
      const label = node.label ? visibleText(node.label) : "";
      const content = node.content.map(visibleText).join("");
      return `${label} ${content}`.trim();
    }
    case "Transparent":
      return "";
  }
}

const library = rk.Library.parseBibtex(
  "@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}",
);
const document = new rk.Document(library, rk.Style.load("apa"), { locale: "en-US" });
const result = document.render([new rk.Citation("intro", "doe2024")]);
const text = result.get("intro").tree.map(visibleText).join("");
console.log(text);
```

:::

```text
(Doe, 2024)
```

TypeScript examples run in Node.js. In a browser, [initialize RefKit](/guides/browser#initialize-the-module) before creating the library.

Use `.text` when the application needs the prepared plain-text result. A tree consumer can retain or transform specific nodes, create its own elements, or connect the output to a source inspector.

## Apply formatting and layout

Text and link nodes carry `formatting`. Map its finite values to the host's formatting system, such as `Italic` to an italic text run and `SmallCaps` to a small-caps style. An element's `display` describes a layout role such as `LeftMargin` or `RightInline`.

A bibliography node separates `label` from `content`. Render the label once, then the content. Read `result.bibliography.layout` for hanging indent, second-field alignment, line spacing, and entry spacing. Apply these settings to the bibliography container or its paragraph styles.

[Data Shapes](/reference/data-shapes) lists every field and accepted formatting value.

## Retain source identity

An element's `meta` can identify its bibliography entry, cite-item index, name role, or name index. Read the metadata `kind` before accessing fields specific to that shape. The bibliography entry's `key` connects rendered output to `library.get(key)`.

`Transparent` nodes carry a citation index and formatting metadata. They contribute no visible text.

## Create HTML safely

`Rendered.html` supplies escaped HTML with allowed links. A custom HTML renderer owns its escaping and URL policy:

- Escape text, markup values, and attribute values.
- Create links for `http`, `https`, and `mailto` URLs when matching RefKit's HTML policy.
- Render other URL schemes as visible text.

A tree `Markup` node is data, not permission to insert trusted HTML. Keep node handling explicit when mapping it into a browser framework or document format.
