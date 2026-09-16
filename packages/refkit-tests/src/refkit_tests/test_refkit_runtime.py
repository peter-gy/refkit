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


def test_rendered_wrappers_return_detached_trees() -> None:
    library = rk.Library.parse_bibtex(BIBTEX)
    rendered = rk.Document(library, rk.Style.load("apa")).render([rk.Citation("a", "doe2024")])
    citation = rendered["a"]
    expected_citation = citation.tree
    detached_citation = rendered.citations["a"].tree
    assert detached_citation == expected_citation
    detached_citation.clear()
    assert rendered["a"].tree == expected_citation

    bibliography = rendered.bibliography
    expected_bibliography = bibliography.tree
    detached_bibliography = rendered.bibliography.tree
    assert detached_bibliography == expected_bibliography
    entry = detached_bibliography[0]
    assert entry["kind"] == "bibliography-entry"
    entry["key"] = "changed"
    entry["content"].clear()
    del rendered
    assert bibliography.tree == expected_bibliography
    assert citation.tree == expected_citation


def test_refkit_validation_uses_owned_reports() -> None:
    library = rk.Library.from_records(
        [{"key": "a", "entry_type": "Misc", "identifiers": {"isbn": "bad"}}]
    )
    assert library.validate()["issues"][0]["code"] == "invalid_identifier"
    report = rk.BibDocument.parse("@article{a,title={A},doi={bad}}").validate()
    assert report["profile"] == "biblatex"
    assert not report["valid"]
    assert any(issue["target"]["span"] is not None for issue in report["issues"])
    with pytest.raises(rk.ParseError) as failure:
        rk.BibDocument.parse("@misc{a,date={123456X}}").validate()
    assert failure.value.diagnostics[0]["code"] == "invalid_field"


def test_refkit_bulk_lookup_preserves_order_and_detached_records() -> None:
    library = rk.Library.parse_bibtex(BIBTEX + "@book{other,title={Café}}")
    records = library.get_many(key for key in ["other", "doe2024", "other"])
    assert [record["key"] for record in records] == ["other", "doe2024", "other"]
    records[0]["identifiers"]["custom"] = "changed"
    assert "custom" not in records[2]["identifiers"]
    assert "custom" not in library["other"]["identifiers"]
    assert library.get_many([]) == []
    with pytest.raises(KeyError, match="missing"):
        library.get_many(key for key in ["other", "missing"])


def test_refkit_codecs_preserve_records_and_report_loss() -> None:
    report = rk.convert(BIBTEX, source_format="biblatex", target_format="csl-json", loss="error")
    restored = rk.decode(report["text"], format="csl-json", loss="error")["library"]
    assert restored.keys() == ["doe2024"]
    assert (
        rk.Document(restored, rk.Style.load("apa")).render([rk.Citation("a", "doe2024")])["a"].text
        == "(Doe, 2024)"
    )
    source = '[{"id":"a","type":"book","issued":{"date-parts":[[2020],[2024]]}}]'
    library = rk.decode(source, format="csl-json")["library"]
    with pytest.raises(rk.ConversionError) as failure:
        rk.encode(library, format="hayagriva", loss="error")
    assert any(issue["lossy"] for issue in failure.value.issues)


def test_refkit_duplicate_review_and_accepted_merge_plan() -> None:
    source = (
        "@string{press={Press}}@book{a,title={A},doi={10.1234/work}}"
        "@book{b,title={A},doi={10.1234/work},publisher=press}@misc{c,crossref={b}}"
    )
    document = rk.BibDocument.parse(source)
    assert len(document.find_duplicates(rules=["doi"])["groups"]) == 1
    plan = document.plan_merge([0, 1], retain=0)
    patch = plan["patch"]
    assert patch is not None
    updated = document.apply_patch(patch)["document"]
    assert updated.resolve()[0]["fields"]["publisher"] == "Press"
    assert updated.resolve()[1]["fields"]["crossref"] == "a"
    assert document.to_bibtex() == source


def test_refkit_raw_edit_and_error_contracts() -> None:
    raw = rk.BibDocument.parse(BIBTEX)
    field = raw.entries["doe2024"].fields["title"]
    updated = raw.apply_patch(
        [
            {
                "kind": "set_field",
                "entry_id": field.entry_id,
                "field_id": field.id,
                "value": "Browser Citations",
            }
        ]
    )["document"]

    assert "Browser Citations" in updated.to_bibtex()
    assert field.value == "Fast Citations"
    assert field.name == "title"
    assert field.span == raw.entries["doe2024"].fields["title"].span
    assert "Fast Citations" in repr(field)

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


def test_refkit_citation_purposes_and_supplied_style_parent() -> None:
    library = rk.Library.parse_bibtex(BIBTEX)
    catalog = rk.Style.list()
    apa = next(style for style in catalog if "apa" in [style["name"], *style["aliases"]])
    document = rk.Document(library, rk.Style.load(apa["name"]), locale="en-US")
    assert (
        document.render([rk.Citation("sentence", rk.Cite("doe2024", purpose="prose"))])[
            "sentence"
        ].text
        == "Doe (2024)"
    )
    child = """<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" default-locale="de-DE">
    <info><title>Child</title><id>https://example.com/child</id>
    <link rel="independent-parent" href="https://example.com/parent"/></info></style>"""
    parent = """<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
    <info><title>Parent</title><id>https://example.com/parent</id></info>
    <locale xml:lang="de-DE"><terms><term name="page">Seite</term></terms></locale>
    <locale xml:lang="en-US"><terms><term name="page">page</term></terms></locale>
    <citation><layout><text term="page"/></layout></citation></style>"""
    style = rk.Style.from_xml(child, parent_xml=parent)
    assert style.title == "Child"
    assert style.csl_id == "https://example.com/child"
    requests = [rk.Citation("term", "doe2024")]
    assert rk.Document(library, style).render(requests)["term"].text == "Seite"
    assert rk.Document(library, style, locale="en-US").render(requests)["term"].text == "page"
    with pytest.raises(ValueError, match="supply parent XML"):
        rk.Style.from_xml(child)
    with pytest.raises(ValueError, match="expected"):
        rk.Style.from_xml(
            child,
            parent_xml=parent.replace("https://example.com/parent", "https://example.com/wrong"),
        )
