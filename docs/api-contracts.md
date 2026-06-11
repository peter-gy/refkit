# API Contracts

Refkit exposes two bibliography models:

| Model | Use it for | Source APIs |
| --- | --- | --- |
| `Library` | Rendering, selection, normalized entry access, and bulk export. | `Library.read(path)`, `Library.parse(source, format=...)` |
| `BibDocument` | Raw `.bib` inspection and field edits that keep comments, strings, preambles, malformed blocks, order, and byte spans. | `BibDocument.read(path)`, `BibDocument.parse(source)` |

## One-Off Helpers

`cite(source, item, style="apa", locale="en-US")` and `bibliography(source, style="apa", locale="en-US")` read `source` as a file path. Use `Library.parse` and `Document` when the bibliography source is already in memory.

```python
import refkit as rk

library = rk.Library.parse("@article{doe2024, title={Fast Citations}, year={2024}}")
doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

print(doc.cite("doe2024").text)
```

`cite` accepts the same citation group shapes as `Document.cite`: a key string, a `Cite` object, or an iterable of key strings and `Cite` objects.

```python
rk.cite("refs.bib", ("doe2024", rk.Cite("roe2022", locator="12", label="page")))
```

## Rendered Output

`Rendered.text` returns plain text. `Rendered.html` returns escaped HTML. `Rendered.tree` and `Rendered.to_tree()` return a list of dictionaries.

Citation trees contain render nodes:

| Node kind | Required keys |
| --- | --- |
| `Text` | `kind`, `text`, `formatting` |
| `Element` | `kind`, `display`, `meta`, `children` |
| `Markup` | `kind`, `value` |
| `Link` | `kind`, `text`, `url`, `formatting` |
| `Transparent` | `kind`, `cite_idx`, `format` |

Bibliography trees contain entries:

| Key | Meaning |
| --- | --- |
| `kind` | Always `bibliography-entry`. |
| `key` | The citation key for the bibliography item. |
| `first_field` | The label or first-field node when the style emits one, otherwise `None`. |
| `children` | Render nodes for the visible bibliography entry. |

`formatting` dictionaries contain `font_style`, `font_variant`, `font_weight`, `text_decoration`, and `vertical_align` string values.

`Element.meta` is a stable category string such as `Entry`, `Name`, `Names`, `Text`, or `CitationNumber`. It does not include upstream debug payloads such as entry indexes or expanded person records.

## Projection Rows

`Library.project(fields=None, keys=None)` returns a list of dictionaries. Supported fields are `key`, `entry_type`, `type`, `title`, `doi`, and `volume`. `title`, `doi`, and `volume` may be `None`.

```python
import refkit as rk

library = rk.Library.read("refs.bib")
rows = library.project(["key", "type", "title"])
```

## Raw BibTeX Blocks

`BibDocument.blocks` returns source-order dictionaries. Every block has `kind` and `span`, where `span` is a two-item byte-offset list.

| Block kind | Extra keys |
| --- | --- |
| `whitespace` | None |
| `comment` | `raw` |
| `preamble` | `value` |
| `string` | `key`, `value` |
| `entry` | `id`, `key` |
| `failed` | `raw`, `error` |
| `other` | `raw` |

`BibDocument.failed_blocks` returns only `failed` block dictionaries.

Direct lookup requires an unambiguous key:

```python
raw = rk.BibDocument.read("refs.bib")

for entry in raw.entries.get_all("doe2024"):
    print(entry.key, entry.span)
```

Use `fields.get_all(name)` when an entry contains duplicate field names.

## Errors

| API | Error | Trigger |
| --- | --- | --- |
| `Library.read(path)` | `RefkitError` | Unsupported file extension. |
| `Library.read(path, strict=True)` | `RefkitError` | Exact BibTeX parse failure. |
| `Library.parse(source, format=...)` | `RefkitError` | Unsupported format. |
| `Library.parse(source, format=..., strict=True)` | `RefkitError` | Exact BibTeX parse failure. |
| `Library.project(fields=..., keys=...)` | `TypeError` | `fields` or `keys` is a string or is not iterable. |
| `Library.project(fields=...)` | `ValueError` | A projection field is not one of `key`, `entry_type`, `type`, `title`, `doi`, or `volume`. |
| `Library.project(keys=...)` | `KeyError` | A requested entry key is absent from the `Library`. |
| `Library.select(selector)` | `ValueError` | Invalid Hayagriva selector. |
| `Style.load(name)` | `ValueError` | Unknown bundled style name. |
| `Style.from_xml(xml)` | `ValueError` | Invalid CSL XML or dependent style input. |
| `Style.from_path(path)` | `RefkitError`, `ValueError` | File read failure or invalid CSL XML. |
| `Locale.load(code)` | `ValueError` | Unknown bundled locale code. |
| `Document.cite(item)` | `MissingReferenceError` | A citation key is absent from the `Library`. The failed call does not append to document state. |
| `Document.cite(Cite(..., label=...))` | `ValueError` | Unknown locator label. |
| `BibDocument.write()` | `ValueError` | The document came from `BibDocument.parse` and no output path was passed. |
| `BibField.value = ...` | `ValueError` | The replacement value cannot be represented safely with the original delimiter mode. |
| `raw.entries[key]` or `raw.entries.get(key)` | `RefkitError` | The raw document contains duplicate entry keys. |
| `entry.fields[name]` or `entry.fields.get(name)` | `RefkitError` | The raw entry contains duplicate field names. |
