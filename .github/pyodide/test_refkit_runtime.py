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
