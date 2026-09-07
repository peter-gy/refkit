from __future__ import annotations

import pytest

import refkit as rk

BIBTEX = """
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  journal = {Journal of Citation Tests},
  year = {2024}
}
"""


def test_refkit_parse_tidy_and_render_contracts() -> None:
    library = rk.Library.parse_bibtex(BIBTEX)
    document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
    rendered = document.render([rk.Citation("intro", "doe2024")])
    tidied = rk.tidy_bibtex(BIBTEX, options=rk.TidyOptions(sort_fields=True))

    assert library.keys() == ["doe2024"]
    assert rendered["intro"].text == "(Doe, 2024)"
    assert rendered.bibliography.text
    assert tidied.bibtex.startswith("@article{doe2024,")
    assert tidied.count == 1


def test_refkit_raw_edit_and_error_contracts() -> None:
    raw = rk.BibDocument.parse(BIBTEX)
    raw.entries["doe2024"].fields["title"].value = "Browser Citations"

    assert "Browser Citations" in raw.to_bibtex()

    library = rk.Library.parse_bibtex(BIBTEX)
    document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
    with pytest.raises(rk.MissingReferenceError):
        document.render([rk.Citation("missing", "missing-key")])


def test_refkit_cyclic_input_returns_parse_diagnostics() -> None:
    with pytest.raises(rk.ParseError) as failure:
        rk.Library.parse_bibtex("@book{loop,title={Loop},crossref={loop}}")

    diagnostic = failure.value.diagnostics[0]
    assert diagnostic["code"] == "cyclic_reference"
    assert diagnostic["entry"] == "loop"
    assert diagnostic["field"] == "crossref"


def test_refkit_style_preparation_bounds_xml_attributes() -> None:
    xml = """<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
      <info><title>Boundary</title><id>https://example.com/boundary</id></info>
      <citation><layout><text value="ok"/></layout></citation>
    </style>"""
    assert rk.Style.from_xml(xml).title == "Boundary"
    attributes = " ".join(f'xmlns:n{i}="urn:test:{i}"' for i in range(257))

    with pytest.raises(ValueError, match="256 attributes"):
        rk.Style.from_xml(xml.replace("<style ", f"<style {attributes} ", 1))
