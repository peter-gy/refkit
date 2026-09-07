# Render citations

A `Document` prepares bibliography data, a [Citation Style Language](https://citationstyles.org/) style, and a locale. Pass related citations in one ordered call. Each call creates fresh citation processor state.

```python
import refkit as rk

source = """@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}
@book{roe2023, author={Roe, Richard}, title={Other Work}, year={2023}}
"""
library = rk.Library.parse_bibtex(source)
requested = ["doe2024"]
missing = [key for key in requested if key not in library]
assert missing == []
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
rendered = document.render(
    [
        rk.Citation("introduction", "doe2024"),
        rk.Citation("detail", rk.Cite("doe2024", locator="12", label="page")),
    ]
)
full = document.full_bibliography()

assert rendered.citation_order == ["introduction", "detail"]
assert rendered["introduction"].text == "(Doe, 2024)"
assert "12" in rendered["detail"].text
assert "Roe" in full.text
assert "Roe" not in rendered.bibliography.text
```

Use `CitationGroup` to combine references within one citation occurrence. A citation ID identifies that occurrence in `RenderedDocument`. For a note style, set `Citation(..., note_number=actual_note_number)` to the surrounding document's note number.

Use `.text` for inspection, `.html` for HTML consumers, and `.tree` for structured formatting and link metadata. Bibliography `.layout` carries spacing and alignment requirements. The cited bibliography contains references processed by the render call. `full_bibliography()` includes every library entry.

Load custom styles with `Style.from_xml(xml)` or `Style.from_path(path)`. A missing reference raises `MissingReferenceError` for the whole call. Inspect requested keys first and return a bounded candidate sample when resolving a mismatch.

For a custom title-based style, supply Citation Style Language XML:

```python
import refkit as rk

style = rk.Style.from_xml("""<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info><title>Titles</title><id>https://example.org/titles</id>
    <updated>2024-01-01T00:00:00+00:00</updated></info>
  <citation><layout><text variable="title"/></layout></citation>
  <bibliography hanging-indent="true"><layout><text variable="title"/></layout></bibliography>
</style>""")
library = rk.Library.parse_bibtex("@book{work, title={Bibliographies}}")
document = rk.Document(library, style)
rendered = document.render([rk.Citation("first", "work")])
assert rendered["first"].text == "Bibliographies"
assert rendered.bibliography.layout["hanging_indent"] is True
```
