from __future__ import annotations

import json
from pathlib import Path

import pytest

import refkit as rk

FIXTURES = Path(__file__).parent / "fixtures"


def test_public_document_example_renders_text_html_and_tree() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    style = rk.Style.load("apa")

    doc = rk.Document(library, style, locale="en-US")
    first = doc.cite("doe2024")
    second = doc.cite([rk.Cite("doe2024", locator="12", label="page"), "roe2022"])
    bibliography = doc.bibliography()
    entry = library["doe2024"]

    assert "Doe" in first.text
    assert entry.volume is None or isinstance(entry.volume, str)
    assert entry.doi == "10.1234/refkit.2024"
    assert second.text
    assert bibliography.text
    assert "<div" in bibliography.html
    assert isinstance(first.tree, list)
    assert first.to_text() == first.text
    assert first.to_html() == first.html
    assert first.to_tree() == first.tree


def test_document_accepts_iterables_for_citation_groups() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    rendered = doc.cite(key for key in ["doe2024", "roe2022"])

    assert rendered.text
    assert doc.bibliography().text


def test_one_off_helpers_render_citation_and_bibliography() -> None:
    citation = rk.cite(FIXTURES / "basic.bib", "doe2024", style="ieee")
    bibliography = rk.bibliography(FIXTURES / "basic.bib", style="chicago-author-date")

    assert citation.text
    assert bibliography.text
    assert bibliography.html
    assert "Doe" in bibliography.text
    assert "Roe" in bibliography.text


def test_one_off_helpers_accept_loaded_style_objects() -> None:
    style = rk.Style.load("apa")

    citation = rk.cite(FIXTURES / "basic.bib", "doe2024", style=style)
    bibliography = rk.bibliography(FIXTURES / "basic.bib", style=style)

    assert "Doe" in citation.text
    assert "Doe" in bibliography.text


def test_document_bibliography_all_renders_uncited_library_entries() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    cited = doc.bibliography()
    full = doc.bibliography(all=True)

    assert cited.text == ""
    assert cited.html == ""
    assert "Doe" in full.text
    assert "Roe" in full.text


def test_library_parse_accepts_source_strings_and_mapping_helpers() -> None:
    library = rk.Library.parse(
        """@article{inline,
  author = {Doe, Jane},
  title = {Inline Source},
  year = {2024}
}
""",
    )

    assert library
    assert not library.is_empty()
    assert library.keys() == ["inline"]
    entry = library.get("inline")
    assert entry is not None
    assert entry.title == "Inline Source"
    assert library.get("missing") is None
    assert [entry.key for entry in library.get_many(["inline"])] == ["inline"]
    assert library.get_many(["inline"])[0].title == "Inline Source"
    with pytest.raises(KeyError):
        library.get_many(["missing"])
    with pytest.raises(TypeError, match="keys must be an iterable"):
        library.get_many("inline")
    assert library.project(["key", "title", "doi", "volume"]) == [
        {"key": "inline", "title": "Inline Source", "doi": None, "volume": None}
    ]
    assert library.project(["key", "title"], keys=["inline"]) == [
        {"key": "inline", "title": "Inline Source"}
    ]

    with pytest.raises(KeyError):
        library.project(["key"], keys=["missing"])
    with pytest.raises(ValueError, match="unsupported projection field"):
        library.project(["unknown"])

    yaml_library = rk.Library.parse((FIXTURES / "parent.yaml").read_text(), format="yaml")
    matches = yaml_library.select("article > periodical[volume]")

    assert matches[0].key == "doe2024"
    assert matches[0].volume == "12"
    assert yaml_library.project(["key", "title", "volume"], keys=["doe2024"]) == [
        {
            "key": "doe2024",
            "title": "Refkit for Bibliographies",
            "volume": "12",
        }
    ]


def test_version_and_missing_module_attribute() -> None:
    assert rk.__version__ == "0.0.0"

    missing_attribute = "does_not_exist"
    with pytest.raises(AttributeError, match="has no attribute"):
        getattr(rk, missing_attribute)


def test_rendered_html_escapes_bibliography_data(tmp_path: Path) -> None:
    source = tmp_path / "xss.bib"
    source.write_text(
        """@article{xss,
  author = {Doe, Jane},
  title = {<script>alert(1)</script>},
  year = {2024}
}
""",
    )

    rendered = rk.bibliography(source, style="apa")

    assert "<script>" not in rendered.html
    assert "&lt;script&gt;" in rendered.html


def test_rendered_html_does_not_emit_unsafe_link_schemes(tmp_path: Path) -> None:
    source = tmp_path / "unsafe-link.bib"
    source.write_text(
        """@online{unsafe,
  author = {Doe, Jane},
  title = {Unsafe Link},
  year = {2024},
  url = {javascript:alert(1)}
}
""",
    )

    rendered = rk.bibliography(source, style="apa")

    assert 'href="javascript:alert(1)"' not in rendered.html
    assert "javascript:alert(1)" in rendered.html
    tree_json = json.dumps(rendered.tree)
    assert '"kind": "Link"' not in tree_json
    assert "javascript:alert(1)" in tree_json


def test_rendered_html_preserves_csl_formatting() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    doc.cite("doe2024")
    rendered = doc.bibliography()

    assert "<i>Journal of Citation Systems</i>" in rendered.html
    assert '<a href="https://doi.org/10.1234/refkit.2024">' in rendered.html


def test_bibliography_text_and_tree_include_second_field_labels() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("ieee"), locale="en-US")

    doc.cite("doe2024")
    rendered = doc.bibliography()

    assert rendered.text.startswith("[1]")
    assert "[1]" in json.dumps(rendered.tree)


def test_library_reads_yaml_and_selects_parent_periodical() -> None:
    library = rk.Library.read(FIXTURES / "parent.yaml")
    matches = library.select("article > periodical[volume]")

    assert len(matches) == 1
    assert matches[0].key == "doe2024"
    assert matches[0].title == "Refkit for Bibliographies"
    assert matches[0].parent is not None
    assert matches[0].parent.title == "Journal of Citation Systems"


def test_library_reads_yml() -> None:
    library = rk.Library.read(FIXTURES / "parent.yml")

    assert "doe2024" in library
    assert library["doe2024"].title == "Refkit for Bibliographies"


def test_library_reads_hayagriva_yaml_schema_and_selectors(tmp_path: Path) -> None:
    source = (FIXTURES / "hayagriva-rich.yaml").read_text(encoding="utf-8")
    library = rk.Library.read(FIXTURES / "hayagriva-rich.yaml")

    assert library.keys() == ["zygos", "kinetics", "wwdc-network"]
    assert [entry.key for entry in library.select("article > periodical[volume]")] == ["kinetics"]
    assert [entry.key for entry in library.select("article > (conference & video)")] == [
        "wwdc-network"
    ]
    projected = library.project(["key", "title", "doi", "volume"], keys=["kinetics"])
    assert projected == [
        {
            "key": "kinetics",
            "title": (
                "Kinetics and luminescence of the excitations of a nonequilibrium "
                "polariton condensate"
            ),
            "doi": "10.1103/PhysRevB.102.165126",
            "volume": "102",
        }
    ]

    exported = {entry["key"]: entry for entry in library.to_dicts()}
    assert exported["zygos"]["parent"]["type"] == "proceedings"
    assert exported["wwdc-network"]["parent"][1]["type"] == "video"
    assert exported["wwdc-network"]["parent"][1]["url"]["date"] == "2020-09-17"

    yml_path = tmp_path / "hayagriva-rich.yml"
    yml_path.write_text(source, encoding="utf-8")
    assert rk.Library.read(yml_path).keys() == library.keys()
    assert rk.Library.parse(source, format="yaml").keys() == library.keys()
    assert rk.Library.parse(source, format="yml").keys() == library.keys()


def test_library_reads_bibtex_and_biblatex_aliases() -> None:
    source = (FIXTURES / "typst-biblatex.bib").read_text(encoding="utf-8")
    read_library = rk.Library.read(FIXTURES / "typst-biblatex.bib")

    assert read_library["biblatex2023"].title == "The biblatex Package"
    assert read_library["arrgh"].parent is not None
    assert read_library["arrgh"].parent.title == "Journal of Political Economy"
    assert read_library["arrgh"].volume == "115"
    assert read_library["tolkien54"].parent is not None
    assert read_library["tolkien54"].parent.title == "The Lord of the Rings"

    for format_name in ["bib", "bibtex", "biblatex"]:
        library = rk.Library.parse(source, format=format_name)
        assert library["arrgh"].volume == "115"
        assert library.project(["key", "title", "volume"], keys=["tolkien54"]) == [
            {
                "key": "tolkien54",
                "title": "The Fellowship of the Ring",
                "volume": "1",
            }
        ]


def test_library_reports_unsupported_read_extension_and_parse_format(tmp_path: Path) -> None:
    source = tmp_path / "refs.json"
    source.write_text("{}", encoding="utf-8")

    with pytest.raises(rk.RefkitError, match='unsupported bibliography extension "json"'):
        rk.Library.read(source)

    with pytest.raises(rk.RefkitError, match='unsupported bibliography format "json"'):
        rk.Library.parse("{}", format="json")


def test_style_and_locale_loaders_cover_supported_sources() -> None:
    xml = (FIXTURES / "refkit-note.csl").read_text(encoding="utf-8")

    bundled = rk.Style.load("apa")
    from_xml = rk.Style.from_xml(xml)
    from_path = rk.Style.from_path(FIXTURES / "refkit-note.csl")
    locale = rk.Locale.load("en-US")
    library = rk.Library.read(FIXTURES / "basic.bib")
    document = rk.Document(library, bundled, locale=locale)

    assert bundled.id == "apa"
    assert bundled.title == "APA Style 7th edition"
    assert from_xml.id == "xml"
    assert from_xml.title == "Refkit Note Fixture"
    assert from_path.id.endswith("refkit-note.csl")
    assert from_path.title == "Refkit Note Fixture"
    assert locale.code == "en-US"
    assert "Doe" in document.cite("doe2024").text

    with pytest.raises(ValueError, match="unknown bundled style"):
        rk.Style.load("unknown-style")
    with pytest.raises(ValueError, match="invalid CSL XML"):
        rk.Style.from_xml("<style/>")
    with pytest.raises(ValueError, match="unknown bundled locale"):
        rk.Locale.load("zz-ZZ")


def test_library_non_strict_keeps_valid_bibtex_entries_with_diagnostics(tmp_path: Path) -> None:
    source = tmp_path / "mixed.bib"
    source.write_text(
        """@article{valid,
  author = {Doe, Jane},
  title = {Kept Entry},
  year = {2024}
}

@broken{missing,
  title = {No close}
""",
    )

    with pytest.raises(rk.RefkitError):
        rk.Library.read(source)

    library = rk.Library.read(source, strict=False, diagnostics=True)

    assert library.keys() == ["valid"]
    assert library["valid"].title == "Kept Entry"
    assert library.diagnostics
    assert "ignored malformed BibTeX block" in library.diagnostics[0]


def test_library_non_strict_recovers_entries_after_unclosed_block(tmp_path: Path) -> None:
    source = tmp_path / "recovery.bib"
    source.write_text(
        """@article{before,
  author = {Doe, Jane},
  title = {Before},
  year = {2023}
}

@broken{missing,
  title = {No close}

@article{after,
  author = {Roe, Richard},
  title = {After},
  year = {2024}
}
""",
    )

    library = rk.Library.read(source, strict=False, diagnostics=True)

    assert library.keys() == ["before", "after"]
    assert library["after"].title == "After"
    assert "ignored malformed BibTeX block" in library.diagnostics[0]


def test_library_non_strict_recovers_entry_after_malformed_at_line(tmp_path: Path) -> None:
    source = tmp_path / "bad-at-line.bib"
    source.write_text(
        """@bad
@article{valid,
  author = {Doe, Jane},
  title = {Kept Entry},
  year = {2024}
}
""",
    )

    library = rk.Library.read(source, strict=False, diagnostics=True)

    assert library.keys() == ["valid"]
    assert "ignored malformed BibTeX block" in library.diagnostics[0]


def test_library_non_strict_drops_closed_malformed_entries(tmp_path: Path) -> None:
    source = tmp_path / "closed-malformed.bib"
    source.write_text(
        """@article{valid,
  author = {Doe, Jane},
  title = {Kept Entry},
  year = {2024}
}

@article{bad,
  title {Missing equals},
  year = {2024}
}
""",
    )

    library = rk.Library.read(source, strict=False, diagnostics=True)

    assert library.keys() == ["valid"]
    assert library.diagnostics
    assert "ignored malformed BibTeX block" in library.diagnostics[0]


def test_library_non_strict_drops_missing_separator_after_bare_value(tmp_path: Path) -> None:
    source = tmp_path / "missing-separator.bib"
    source.write_text(
        """@article{bad,
  year = 2024
  title = {Bad}
}

@article{valid,
  author = {Doe, Jane},
  title = {Kept Entry},
  year = {2024}
}
""",
    )

    library = rk.Library.read(source, strict=False, diagnostics=True)

    assert library.keys() == ["valid"]
    assert "ignored malformed BibTeX block" in library.diagnostics[0]


def test_library_non_strict_drops_missing_field_values(tmp_path: Path) -> None:
    source = tmp_path / "missing-value.bib"
    source.write_text(
        """@article{bad,
  title = ,
  year = {2024}
}

@article{valid,
  author = {Doe, Jane},
  title = {Kept Entry},
  year = {2024}
}
""",
    )

    library = rk.Library.read(source, strict=False, diagnostics=True)

    assert library.keys() == ["valid"]
    assert "ignored malformed BibTeX block" in library.diagnostics[0]


def test_library_non_strict_drops_entries_missing_key_comma(tmp_path: Path) -> None:
    source = tmp_path / "missing-key-comma.bib"
    source.write_text(
        """@article{bad
  title = {Bad},
  year = {2024}
}

@article{valid,
  author = {Doe, Jane},
  title = {Kept Entry},
  year = {2024}
}
""",
    )

    library = rk.Library.read(source, strict=False, diagnostics=True)

    assert library.keys() == ["valid"]
    assert "ignored malformed BibTeX block" in library.diagnostics[0]


def test_library_non_strict_drops_malformed_field_identifiers(tmp_path: Path) -> None:
    source = tmp_path / "bad-field-name.bib"
    source.write_text(
        """@article{bad,
  -title = {Bad},
  year = {2024}
}

@article{valid,
  author = {Doe, Jane},
  title = {Kept Entry},
  year = {2024}
}
""",
    )

    library = rk.Library.read(source, strict=False, diagnostics=True)

    assert library.keys() == ["valid"]
    assert "ignored malformed BibTeX block" in library.diagnostics[0]


def test_library_non_strict_drops_malformed_unsafe_bare_values(tmp_path: Path) -> None:
    source = tmp_path / "unsafe-bare.bib"
    source.write_text(
        """@article{bad,
  title = Bad{Thing},
  year = {2024}
}

@article{valid,
  author = {Doe, Jane},
  title = {Kept Entry},
  year = {2024}
}
""",
    )

    library = rk.Library.read(source, strict=False, diagnostics=True)

    assert library.keys() == ["valid"]
    assert "ignored malformed BibTeX block" in library.diagnostics[0]


def test_library_non_strict_drops_malformed_string_definitions(tmp_path: Path) -> None:
    source = tmp_path / "bad-string.bib"
    source.write_text(
        """@string{badstring}
@string{ = "Journal"}
@string{bad = {A} trailing}

@article{valid,
  author = {Doe, Jane},
  title = {Kept Entry},
  year = {2024}
}
""",
    )

    library = rk.Library.read(source, strict=False, diagnostics=True)

    assert library.keys() == ["valid"]
    assert len(library.diagnostics) == 3
    assert all("ignored malformed BibTeX block" in item for item in library.diagnostics)


def test_missing_reference_raises_structured_error() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    with pytest.raises(rk.MissingReferenceError, match="missing-key"):
        doc.cite("missing-key")

    assert "Doe" in doc.cite("doe2024").text


def test_raw_bib_document_preserves_blocks_and_writes_field_edit(tmp_path: Path) -> None:
    raw = rk.BibDocument.read(FIXTURES / "raw.bib")

    assert raw.comments[0].startswith("% library comment")
    assert raw.preamble == "BibTeX preamble"
    assert raw.strings["jcs"] == "Journal of Citation Systems"
    failed_blocks = raw.failed_blocks
    assert failed_blocks[0]["kind"] == "failed"
    assert "closing delimiter" in failed_blocks[0]["error"]
    assert raw.entries["doe2024"].span[0] < raw.entries["doe2024"].span[1]

    raw.entries["doe2024"].fields["title"].value = "Corrected title"
    output = tmp_path / "updated.bib"
    raw.write(output)

    text = output.read_text()
    assert "% library comment" in text
    assert "@preamble" in text
    assert "@string" in text
    assert "@broken" in text
    assert "Corrected title" in text
    assert "Old Title" not in text
    assert "journal = jcs" in text
    assert "journal = {jcs}" not in text


def test_raw_bib_document_preserves_typst_biblatex_blocks(tmp_path: Path) -> None:
    raw = rk.BibDocument.read(FIXTURES / "typst-raw.bib")

    assert raw.comments[0].startswith("@comment")
    assert any(comment.startswith("% Comments before") for comment in raw.comments)
    assert raw.strings["benchjournal"] == "Journal of Citation Benchmarks"
    assert raw.preamble == '"Reference " # "fixture"'
    assert raw.entries.keys() == ["fischer2022equivalence", "roes2003belief"]
    assert raw.entries["roes2003belief"].span[0] < raw.entries["roes2003belief"].span[1]
    assert raw.failed_blocks[0]["kind"] == "failed"
    assert "field author is missing '='" in raw.failed_blocks[0]["error"]

    raw.entries["roes2003belief"].fields["title"].value = "Edited belief title"
    output = tmp_path / "typst-raw-out.bib"
    raw.write(output)

    text = output.read_text(encoding="utf-8")
    assert "@comment{thisdoesntmatter" in text
    assert "% Comments before the entry work" in text
    assert "@string{benchjournal" in text
    assert '@preamble{"Reference " # "fixture"}' in text
    assert "Edited belief title" in text
    assert "Belief in moralizing gods" not in text
    assert "@inproceedings{conigliocorbalan" in text
    assert "author    {Marcelo Coniglio and Maria Corbalan}" in text


def test_raw_bib_document_parse_accepts_source_strings_and_mapping_helpers() -> None:
    raw = rk.BibDocument.parse(
        """% inline comment
@article{inline,
  title = {Inline Raw}
}

@broken{missing
""",
    )

    assert raw.comments == ["% inline comment\n"]
    assert raw.entries
    assert not raw.entries.is_empty()
    entry = raw.entries.get("inline")
    assert entry is not None
    assert entry.key == "inline"
    assert raw.entries.get("missing") is None
    assert raw.entries["inline"].fields
    assert not raw.entries["inline"].fields.is_empty()
    title = raw.entries["inline"].fields.get("title")
    assert title is not None
    assert title.value == "Inline Raw"
    assert raw.entries["inline"].fields.get("missing") is None
    assert raw.failed_blocks

    with pytest.raises(ValueError, match="write path is required"):
        raw.write()


def test_raw_bib_document_accepts_permissive_citation_keys() -> None:
    raw = rk.BibDocument.parse(
        """@article{key+?é,
  title = {Permissive Key}
}

@article{key"q,
  title = {Quoted Key}
}
""",
    )

    assert raw.failed_blocks == []
    assert raw.entries["key+?é"].fields["title"].value == "Permissive Key"
    assert raw.entries['key"q'].fields["title"].value == "Quoted Key"


def test_raw_bib_document_preserves_preamble_expression(tmp_path: Path) -> None:
    source = tmp_path / "preamble-expression.bib"
    source.write_text('@preamble{"A" # "B"}\n')

    raw = rk.BibDocument.read(source)

    assert raw.failed_blocks == []
    assert raw.preamble == '"A" # "B"'


def test_raw_bib_document_accepts_trailing_string_comment(tmp_path: Path) -> None:
    source = tmp_path / "string-comment.bib"
    source.write_text(
        """@string{jcs = "Journal of Citation Systems" % local abbreviation
}
""",
    )

    raw = rk.BibDocument.read(source)

    assert raw.failed_blocks == []
    assert raw.strings["jcs"] == "Journal of Citation Systems"


def test_raw_bib_document_keeps_unmatched_quotes_inside_comment_blocks(tmp_path: Path) -> None:
    source = tmp_path / "comment-quote.bib"
    source.write_text(
        """@comment{reviewed by "anonymous}

@article{kept,
  title = {Kept},
  year = {2024}
}
""",
    )

    raw = rk.BibDocument.read(source)

    assert raw.failed_blocks == []
    assert raw.comments == ['@comment{reviewed by "anonymous}']
    assert raw.entries["kept"].fields["title"].value == "Kept"


def test_raw_helper_classes_are_runtime_exports() -> None:
    assert rk.BibEntry.__name__ == "BibEntry"
    assert rk.BibEntryMap.__name__ == "BibEntryMap"
    assert rk.BibField.__name__ == "BibField"
    assert rk.BibFieldMap.__name__ == "BibFieldMap"


def test_raw_bare_field_edit_wraps_unsafe_value(tmp_path: Path) -> None:
    source = tmp_path / "bare.bib"
    source.write_text(
        """@article{macro,
  journal = jcs,
  title = {Uses Macro}
}
""",
    )

    raw = rk.BibDocument.read(source)
    raw.entries["macro"].fields["journal"].value = "Journal of Citation Systems"
    output = tmp_path / "bare-out.bib"
    raw.write(output)

    text = output.read_text()
    assert "journal = {Journal of Citation Systems}" in text


def test_raw_field_edit_allows_balanced_case_protection_braces(tmp_path: Path) -> None:
    raw = rk.BibDocument.parse(
        """@article{braces,
  braced = {Old},
  quoted = "Old",
  bare = token
}
""",
    )

    raw.entries["braces"].fields["braced"].value = "{NASA} Mission"
    raw.entries["braces"].fields["quoted"].value = "{ESA} Mission"
    raw.entries["braces"].fields["bare"].value = "{JAXA} Mission"
    output = tmp_path / "braces.bib"
    raw.write(output)

    text = output.read_text()
    assert "braced = {{NASA} Mission}" in text
    assert 'quoted = "{ESA} Mission"' in text
    assert "bare = {{JAXA} Mission}" in text


def test_raw_field_edit_allows_protected_quotes_in_quoted_values(tmp_path: Path) -> None:
    raw = rk.BibDocument.parse(
        """@article{quoted,
  title = "Old",
  year = {2024}
}
""",
    )

    raw.entries["quoted"].fields["title"].value = 'A {"quoted"} title'
    output = tmp_path / "quoted-out.bib"
    raw.write(output)

    text = output.read_text()
    assert 'title = {A {"quoted"} title}' in text
    library = rk.Library.read(output)
    assert library["quoted"].title == 'A "quoted" title'


def test_raw_field_edit_rejects_unsafe_delimiters(tmp_path: Path) -> None:
    source = tmp_path / "unsafe.bib"
    source.write_text(
        """@article{unsafe,
  braced = {Original},
  quoted = "Original",
  bare = token
}
""",
    )
    raw = rk.BibDocument.read(source)

    with pytest.raises(ValueError, match="unsafe braced delimiter"):
        raw.entries["unsafe"].fields["braced"].value = "Bad } value"
    with pytest.raises(ValueError, match="unsafe braced delimiter"):
        raw.entries["unsafe"].fields["braced"].value = "Bad { value"
    with pytest.raises(ValueError, match="unsafe braced delimiter"):
        raw.entries["unsafe"].fields["braced"].value = "Bad\\"
    with pytest.raises(ValueError, match="unsafe quoted delimiter"):
        raw.entries["unsafe"].fields["quoted"].value = 'He said "hi"'
    with pytest.raises(ValueError, match="unsafe quoted delimiter"):
        raw.entries["unsafe"].fields["quoted"].value = "Bad\\"
    with pytest.raises(ValueError, match="unsafe quoted delimiter"):
        raw.entries["unsafe"].fields["quoted"].value = "Bad { value"
    with pytest.raises(ValueError, match="unsafe quoted delimiter"):
        raw.entries["unsafe"].fields["quoted"].value = "Bad } value"
    with pytest.raises(ValueError, match="unsafe braced delimiter"):
        raw.entries["unsafe"].fields["bare"].value = "Bad } value"
    with pytest.raises(ValueError, match="unsafe braced delimiter"):
        raw.entries["unsafe"].fields["bare"].value = "Bad { value"
    with pytest.raises(ValueError, match="unsafe braced delimiter"):
        raw.entries["unsafe"].fields["bare"].value = "Bad\\"


def test_raw_field_edit_replaces_whole_concatenated_expression(tmp_path: Path) -> None:
    source = tmp_path / "concat.bib"
    source.write_text(
        """@article{concat,
  title = {Old} # {Subtitle},
  year = {2024}
}
""",
    )

    raw = rk.BibDocument.read(source)
    raw.entries["concat"].fields["title"].value = "New"
    output = tmp_path / "concat-out.bib"
    raw.write(output)

    text = output.read_text()
    assert "title = {New}," in text
    assert "Subtitle" not in text


def test_raw_bib_document_preserves_duplicate_keys_on_write(tmp_path: Path) -> None:
    source = tmp_path / "duplicates.bib"
    source.write_text(
        """@article{same,
  title = {First},
  year = {2023}
}

@article{same,
  title = {Second},
  year = {2024}
}
""",
    )

    raw = rk.BibDocument.read(source)
    raw.entries["same"].fields["title"].value = "Corrected second"
    output = tmp_path / "duplicates-out.bib"
    raw.write(output)

    text = output.read_text()
    assert "title = {First}" in text
    assert "title = {Corrected second}" in text
    assert text.count("@article{same") == 2


def test_raw_bib_document_ignores_comment_delimiters_inside_entries(tmp_path: Path) -> None:
    source = tmp_path / "commented.bib"
    source.write_text(
        """@article{commented,
  % }
  title = {Still Here},
  year = {2024}
}
""",
    )

    raw = rk.BibDocument.read(source)

    assert raw.failed_blocks == []
    assert raw.entries["commented"].fields["title"].value == "Still Here"


def test_raw_bib_document_handles_escaped_and_nested_delimiters(tmp_path: Path) -> None:
    source = tmp_path / "delimiters.bib"
    source.write_text(
        """@article{escaped,
  title = {A \\} B},
  year = {2024}
}

@article(paren,
  title = {A) B},
  year = {2025}
)
""",
    )

    raw = rk.BibDocument.read(source)

    assert raw.failed_blocks == []
    assert raw.entries["escaped"].fields["year"].value == "2024"
    assert raw.entries["paren"].fields["title"].value == "A) B"


def test_raw_bib_document_keeps_quotes_literal_inside_braced_values(tmp_path: Path) -> None:
    source = tmp_path / "quoted-brace.bib"
    source.write_text(
        """@article{quoted,
  title = {A " quoted title},
  year = {2024}
}
""",
    )

    raw = rk.BibDocument.read(source)

    assert raw.failed_blocks == []
    assert raw.entries["quoted"].fields["title"].value == 'A " quoted title'


def test_raw_bib_document_keeps_protected_quotes_inside_quoted_values(tmp_path: Path) -> None:
    source = tmp_path / "protected-quote.bib"
    source.write_text(
        """@article{quoted,
  title = "A {"quoted"} title",
  year = {2024}
}
""",
    )

    raw = rk.BibDocument.read(source)

    assert raw.failed_blocks == []
    assert raw.entries["quoted"].fields["title"].value == 'A {"quoted"} title'


def test_raw_bib_document_keeps_single_protected_quote_inside_quoted_values(tmp_path: Path) -> None:
    source = tmp_path / "protected-single-quote.bib"
    source.write_text(
        """@article{quoted,
  title = "A {"} title",
  year = {2024}
}
""",
    )

    raw = rk.BibDocument.read(source)

    assert raw.failed_blocks == []
    assert raw.entries["quoted"].fields["title"].value == 'A {"} title'


def test_raw_bib_document_allows_inline_comment_after_final_field(tmp_path: Path) -> None:
    source = tmp_path / "final-comment.bib"
    source.write_text(
        """@article{commented,
  title = {Kept} % note
}
""",
    )

    raw = rk.BibDocument.read(source)

    assert raw.failed_blocks == []
    assert raw.entries["commented"].fields["title"].value == "Kept"


def test_raw_bib_document_allows_no_space_comment_after_bare_value(tmp_path: Path) -> None:
    source = tmp_path / "bare-comment.bib"
    source.write_text(
        """@article{commented,
  year = 2024% note
}
""",
    )

    raw = rk.BibDocument.read(source)

    assert raw.failed_blocks == []
    assert raw.entries["commented"].fields["year"].value == "2024"
