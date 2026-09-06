---
description: Read BibTeX, BibLaTeX, or Hayagriva YAML and inspect normalized entries, projections, and selectors.
---

# Parse Bibliographies

`Library` parses bibliography source into normalized `Entry` objects. Use path methods for files and parse methods for in-memory strings.

## Read a file

```python
import refkit as rk

library = rk.Library.read("references.bib")
print(library.keys())
```

`Library.read` selects the parser from the file extension:

| Extension | Input |
| --- | --- |
| `.bib` | BibTeX or BibLaTeX source. |
| `.yaml`, `.yml` | Hayagriva bibliography YAML. |

RefKit decodes UTF-8 first. A file that requires the Windows-1252-compatible fallback receives a parser diagnostic so the encoding decision stays visible.

## Parse an in-memory string

```python
library = rk.Library.parse_bibtex(
    "@article{doe2024, title={Fast Citations}, year={2024}}"
)
```

Use `Library.parse_yaml(source)` for Hayagriva YAML.

## Inspect entries

```python
print(len(library))
print("doe2024" in library)

entry = library["doe2024"]
print(entry.key)
print(entry.entry_type)
print(entry.title)
```

`get(key)` returns `None` for a missing key. Indexing with `library[key]` raises `KeyError`. `get_many(keys)` preserves the requested order and raises when a requested key is absent.

Use `values()` for normalized entries in library order and `is_empty()` when the empty state should be explicit.

## Project dictionary rows

`Library.project` creates Python dictionaries with a stable RefKit field vocabulary:

```python
rows = library.project(["key", "entry_type", "title", "date", "doi", "volume"])
```

Supported fields are `key`, `entry_type`, `type`, `title`, `date`, `doi`, and `volume`. `type` is an alias for the entry type under that output key. Title, date, DOI, and volume values can be `None`.

Pass `keys=` to limit and order the projected entries:

```python
rows = library.project(["key", "title"], keys=["doe2024"])
```

## Select by bibliography structure

`Library.select` uses Hayagriva selectors to match normalized entries and their parent relationships:

```python
periodical_articles = library.select("article > periodical[volume]")
```

Read [Selectors](/reference/selectors) for the supported selector grammar and result behavior.

## Keep recoverable entries

```python
library = rk.Library.read("references.bib", recovery="report")

if library.diagnostics:
    for diagnostic in library.diagnostics:
        print(diagnostic)
```

Use report recovery when the application can show or store diagnostics beside recovered entries. Keep the default error recovery policy when later work requires an exact parse.
