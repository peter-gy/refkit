# API Contracts

Refkit exposes two bibliography models:

| Model | Use it for | Source APIs |
| --- | --- | --- |
| `Library` | Rendering, selection, normalized entry access, and bulk export. | `Library.read(path)`, `Library.parse_bibtex(source)`, `Library.parse_yaml(source)` |
| `BibDocument` | Raw `.bib` inspection, formatting, and field edits that keep comments, strings, preambles, malformed blocks, order, and byte spans. | `BibDocument.read(path)`, `BibDocument.parse(source)`, `BibDocument.tidy(options=...)` |

## One-Off Helpers

`cite(source, citation, style="apa", locale="en-US")` and `full_bibliography(source, style="apa", locale="en-US")` read `source` as a file path. Use `Library.parse_bibtex`, `Library.parse_yaml`, and `Document` when the bibliography source is already in memory.

```python
import refkit as rk

library = rk.Library.parse_bibtex(
    """
@article{doe2024, title={Fast Citations}, year={2024}}
@book{roe2022, title={Batch References}, year={2022}}
"""
)
doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

rendered = doc.render([rk.Citation("intro", "doe2024")])
print(rendered["intro"].text)
```

`Document.render` accepts named `Citation` objects. Each `Citation` has a unique `id` and a citation value: a key string, a `Cite`, or a `CitationGroup`. Use `CitationGroup([...])` when one rendered citation contains multiple items. Raw lists, tuples, and generators are group inputs inside `CitationGroup`, not direct `Document.render` arguments.

```python
doc.render(
    [
        rk.Citation(
            "detail",
            rk.CitationGroup(["doe2024", rk.Cite("roe2022", locator="12", label="page")]),
        )
    ]
)
```

`Document.render([...])` returns `RenderedDocument`. Use `rendered["detail"]` or `rendered.citations["detail"]` for named citations and `rendered.bibliography` for the cited bibliography. Use `Document.cited_bibliography([...])` when only the cited bibliography is needed. Use `Document.full_bibliography()` when every library entry should appear.

## BibTeX Formatting

`tidy_bibtex(source, options=None)` formats a BibTeX string and returns `TidyResult`.

```python
import refkit as rk

options = rk.TidyOptions(sort_fields=True, wrap=88)
result = rk.tidy_bibtex("@ARTICLE{doe2024, pages={6-13}, year={2024}}\n", options=options)

print(result.bibtex)
```

`TidyResult.bibtex` is the formatted BibTeX string. `TidyResult.count` is the number of parsed entries. `TidyResult.warnings` contains `TidyWarning` objects with `code`, `rule`, and `message`.

`BibDocument.to_bibtex()` renders the current raw document state. `BibDocument.tidy(options=None)` formats that rendered state and returns `TidyResult`.

```python
raw = rk.BibDocument.read("refs.bib")
raw.entries["doe2024"].fields["title"].value = "Corrected title"
result = raw.tidy(options=rk.TidyOptions(sort_fields=True))
```

`tidy_file(path, output=None, options=None)` reads a BibTeX file through `BibDocument.read`. It writes `result.bibtex` when `output` is supplied.

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

`Library.project(fields=None, *, keys=None)` returns a list of dictionaries. Supported fields are `key`, `entry_type`, `type`, `title`, `doi`, and `volume`. `title`, `doi`, and `volume` may be `None`.

```python
import refkit as rk

library = rk.Library.read("refs.bib")
rows = library.project(["key", "type", "title"])
```

`Library.read` and `Library.parse_bibtex` use `recovery="error"` by default. Malformed BibTeX raises `RefkitError`. Use `recovery="report"` when a workflow intentionally keeps recoverable entries and reads `library.diagnostics`.

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

Use `raw.entries.unique_keys()` when one key per distinct entry name is needed. Use `raw.entries.occurrence_keys()` when source-order duplicates matter. Use `fields.get_all(name)` and `fields.occurrence_keys()` when an entry contains duplicate field names.

## Errors

| API | Error | Trigger |
| --- | --- | --- |
| `Library.read(path)` | `RefkitError` | Unsupported file extension. |
| `Library.read(path, *, recovery="error")` | `RefkitError` | BibTeX parse failure. |
| `Library.parse_bibtex(source, *, recovery="error")` | `RefkitError` | BibTeX parse failure. |
| `Library.parse_yaml(source)` | `RefkitError` | Invalid Hayagriva YAML. |
| `Library.project(fields=..., keys=...)` | `TypeError` | `fields` or `keys` is a string or is not iterable. |
| `Library.project(fields=...)` | `ValueError` | A projection field is not one of `key`, `entry_type`, `type`, `title`, `doi`, or `volume`. |
| `Library.project(keys=...)` | `KeyError` | A requested entry key is absent from the `Library`. |
| `Library.select(selector)` | `ValueError` | Invalid Hayagriva selector. |
| `Style.load(name)` | `ValueError` | Unknown bundled style name. |
| `Style.from_xml(xml)` | `ValueError` | Invalid CSL XML or dependent style input. |
| `Style.from_path(path)` | `RefkitError`, `ValueError` | File read failure or invalid CSL XML. |
| `Locale.load(code)` | `ValueError` | Unknown bundled locale code. |
| `tidy_bibtex(source)` | `TidySyntaxError` | The raw BibTeX parser records a malformed block. |
| `tidy_bibtex(source, options=...)` | `TidyError` | Key template generation fails. |
| `Document.render(citations)` | `TypeError` | `citations` is not an iterable of `Citation` objects. |
| `CitationGroup([])` | `ValueError` | The group has no citation items. |
| `Document.render(citations)` | `MissingReferenceError` | A citation key is absent from the `Library`. |
| `Document.render([Citation(..., Cite(..., label=...))])` | `ValueError` | Unknown locator label. |
| `BibDocument.write(path)` | `RefkitError` | The output path cannot be written. |
| `BibField.value = ...` | `ValueError` | The replacement value cannot be represented safely with the original delimiter mode. |
| `raw.entries[key]` or `raw.entries.get_unique(key)` | `RefkitError` | The raw document contains duplicate entry keys. |
| `entry.fields[name]` or `entry.fields.get_unique(name)` | `RefkitError` | The raw entry contains duplicate field names. |
