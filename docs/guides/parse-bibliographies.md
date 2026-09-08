---
description: Parse bibliography text, inspect normalized entries, project records, and select entries by structure in Python or TypeScript.
---

# Parse Bibliographies

`Library` parses [BibTeX](https://ctan.org/pkg/bibtex) and [BibLaTeX](https://ctan.org/pkg/biblatex) bibliography text into normalized `Entry` objects.

::: code-group

```python [Python]
import refkit as rk

source = (
    "@article{doe2024, title={Fast Citations}, journal={Citation Tests}, volume={12}, year={2024}}"
)
library = rk.Library.parse_bibtex(source)
print(library.get("doe2024").title)
```

```ts [TypeScript]
import * as rk from "refkit-js";

const source = "@article{doe2024, title={Fast Citations}, journal={Citation Tests}, volume={12}, year={2024}}";
const library = rk.Library.parseBibtex(source);
console.log(library.get("doe2024")!.title);
```

:::

```text
Fast Citations
```

TypeScript examples run in Node.js after [installation](/get-started). In a browser, [initialize RefKit](/guides/browser#initialize-the-module) before parsing. The remaining examples continue with `library`.

For [Hayagriva YAML](https://github.com/typst/hayagriva#file-format), a bibliography format with nested entry relationships, use `Library.parse_yaml(source)` in Python or `Library.parseYaml(source)` in TypeScript.

## Inspect entries

::: code-group

```python [Python]
print(len(library))
print("doe2024" in library)

entry = library["doe2024"]
print(entry.key, entry.entry_type, entry.title)
```

```ts [TypeScript]
console.log(library.size);
console.log(library.has("doe2024"));

const entry = library.get("doe2024")!;
console.log(entry.key, entry.entryType, entry.title);
```

:::

`get(key)` returns `None` in Python or `null` in TypeScript for a missing key. Python indexing raises `KeyError` when the key is absent. `get_many(keys)` / `getMany(keys)` preserves the requested order and raises when a requested key is absent.

Use `values()` for normalized entries in library order. `is_empty()` / `isEmpty()` checks whether the library contains any entries.

## Project records

`Library.project` returns a list of Python dictionaries or an array of TypeScript objects. Select fields and optionally limit and order the entries:

::: code-group

```python [Python]
rows = library.project(["key", "entry_type", "title", "date", "doi", "volume"])
selected_rows = library.project(["key", "title"], keys=["doe2024"])
print(selected_rows[0]["title"])
```

```ts [TypeScript]
const rows = library.project(["key", "entryType", "title", "date", "doi", "volume"]);
const selectedRows = library.project(["key", "title"], { keys: ["doe2024"] });
console.log(selectedRows[0]!.title);
```

:::

`type` is an alias for the entry type under that output key. Title, date, DOI, and volume values can be `None` / `null`. [Data Shapes](/reference/data-shapes) defines the normalized fields.

## Select by bibliography structure

`Library.select` uses Hayagriva selectors to match normalized entries and their parent relationships:

::: code-group

```python [Python]
periodical_articles = library.select("article > periodical[volume]")
print([entry.key for entry in periodical_articles])
```

```ts [TypeScript]
const periodicalArticles = library.select("article > periodical[volume]");
console.log(periodicalArticles.map(entry => entry.key));
```

:::

This selector returns `doe2024`, whose periodical parent has a volume. Read [Selectors](/reference/selectors) for the grammar and result behavior.

## Read a file

Python's path methods and the [Node.js](https://nodejs.org/) filesystem helpers select the parser from the extension. These examples read an existing `references.bib` file:

::: code-group

```python [Python]
file_library = rk.Library.read("references.bib")
print(file_library.keys())
```

```ts [TypeScript]
import { readLibrary } from "refkit-js/node";

const fileLibrary = await readLibrary("references.bib");
console.log(fileLibrary.keys());
```

:::

| Extension | Input |
| --- | --- |
| `.bib` | BibTeX or BibLaTeX source. |
| `.yaml`, `.yml` | Hayagriva bibliography YAML. |

RefKit decodes UTF-8 first. A file that requires the Windows-1252-compatible fallback receives a parser diagnostic so the encoding decision stays visible.

## Keep recoverable entries

Report recovery keeps entries that can be parsed and records diagnostics beside them:

::: code-group

```python [Python]
recovered = rk.Library.parse_bibtex(source + "\n@book{broken", recovery="report")
for diagnostic in recovered.diagnostics:
    print(diagnostic["code"], diagnostic["message"])
```

```ts [TypeScript]
const recovered = rk.Library.parseBibtex(source + "\n@book{broken", { recovery: "report" });
for (const diagnostic of recovered.diagnostics) {
  console.log(diagnostic.code, diagnostic.message);
}
```

:::

The recovered library retains `doe2024` and reports the malformed trailing block. The file readers accept the same recovery option. Keep the default `"error"` policy when subsequent work requires an exact parse. [Recovery](/concepts/parsing-and-recovery) explains the diagnostic fields and recovery boundaries.
