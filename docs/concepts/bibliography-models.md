---
description: Choose Library for normalized behavior or BibDocument for source-preserving BibTeX edits.
---

# Choose a Bibliography Model

`Library` stores normalized citation data. `BibDocument` preserves [BibTeX](https://www.bibtex.org/) source, a text format for bibliography entries. Choose the model from the information the workflow must retain.

## Use `Library` for normalized data

::: code-group

```python [Python]
import refkit as rk

library = rk.Library.parse_bibtex("@article{doe2024, title={Fast Citations}, year={2024}}")
entry = library["doe2024"]
print(entry.key, entry.entry_type, entry.title)
```

```ts [TypeScript]
import * as rk from "refkit-js";

const library = rk.Library.parseBibtex("@article{doe2024, title={Fast Citations}, year={2024}}");
const entry = library.get("doe2024")!;
console.log(entry.key, entry.entryType, entry.title);
```

:::

Both print `doe2024 Article Fast Citations`. TypeScript examples run in Node.js. For browsers, complete [browser initialization](/guides/browser#initialize-the-module) before calling the same APIs.

`Library` owns normalized entries, parent relationships, key lookup, selection, projection, and parser diagnostics. Pass a library to the renderer when a workflow needs citations or a bibliography. Normalization discards source layout such as whitespace and field delimiters.

## Use `BibDocument` for source-preserving edits

Using the `rk` import, parse and edit a title:

::: code-group

```python [Python]
source = """% reviewed by Jane
@article{doe2024,
  title = {Old title},
  year = {2024}
}
"""

document = rk.BibDocument.parse(source)
document.entries["doe2024"].fields["title"].value = "Corrected title"
print(document.to_bibtex())
```

```ts [TypeScript]
const source = `% reviewed by Jane
@article{doe2024,
  title = {Old title},
  year = {2024}
}
`;

const document = rk.BibDocument.parse(source);
document.entries.getUnique("doe2024")!.fields.getUnique("title")!.value = "Corrected title";
console.log(document.toBibtex());
```

:::

The output contains `title = {Corrected title}` and retains the comment, spacing, and year field. `BibDocument` keeps source-order blocks, entry and field occurrences, and byte spans. Field assignments validate the replacement against the original delimiter mode before changing the document.

## Address duplicates by occurrence

An occurrence is one entry or field at one source position. A unique lookup raises `RefkitError` when the name has multiple occurrences. Retrieve the occurrences to choose which one to edit:

::: code-group

```python [Python]
duplicates = rk.BibDocument.parse("""
@article{same, title={First}}
@article{same, title={Second}}
""")
second = duplicates.entries.get_all("same")[1]
second.fields["title"].value = "Updated second title"
print(duplicates.to_bibtex())
```

```ts [TypeScript]
const duplicates = rk.BibDocument.parse(`
@article{same, title={First}}
@article{same, title={Second}}
`);
const second = duplicates.entries.getAll("same")[1]!;
second.fields.getUnique("title")!.value = "Updated second title";
console.log(duplicates.toBibtex());
```

:::

The first title stays `First`. The second becomes `Updated second title`. Entry and field maps expose both unique names and source-order occurrences. See the [Python](/reference/python#raw-bibtex) and [TypeScript](/reference/javascript#raw-bibtex) references for lookup and missing-name behavior.

## Move between the models deliberately

The models own separate state. Parse the edited document's writeback into a new library when rendering must reflect an edit:

::: code-group

```python [Python]
updated = rk.Library.parse_bibtex(document.to_bibtex())
print(updated["doe2024"].title)
```

```ts [TypeScript]
const updated = rk.Library.parseBibtex(document.toBibtex());
console.log(updated.get("doe2024")!.title);
```

:::

Both print `Corrected title`.

Continue with [Parsing and Recovery](/concepts/parsing-and-recovery) or [Edit Raw BibTeX](/guides/edit-bibtex).
