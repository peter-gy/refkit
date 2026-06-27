from __future__ import annotations

import importlib
import sys
import threading
from collections.abc import Callable, Iterator
from importlib import metadata
from pathlib import Path
from time import sleep
from types import ModuleType
from typing import Any, TypeVar, cast

import pytest

import refkit as rk

ROOT = Path(__file__).parent.parent
WORKSPACE = ROOT.parent.parent
FIXTURES = Path(__file__).parent / "fixtures"
ARXIV_FIXTURE = WORKSPACE / "data" / "arxiv-wild" / "references-subset.bib"
T = TypeVar("T")


def _many_bibtex_records(count: int) -> str:
    return "\n\n".join(
        f"""@article{{item{index:04d},
  author = {{Family{index:04d}, Given{index:04d}}},
  title = {{Reference Work {index:04d}}},
  journal = {{Journal of Citation Tests}},
  year = {{2024}},
  volume = {{1}},
  pages = {{1-2}}
}}"""
        for index in range(count)
    )


def _tree_nodes(value: object) -> Iterator[dict[str, Any]]:
    if isinstance(value, list):
        for child in value:
            yield from _tree_nodes(child)
        return
    if not isinstance(value, dict):
        return
    node = cast(dict[str, Any], value)
    yield node
    yield from _tree_nodes(node.get("first_field"))
    yield from _tree_nodes(node.get("children"))


def _run_with_worker_progress(operation: Callable[[], T]) -> T:
    started = threading.Event()
    done = threading.Event()
    ticks: list[None] = []

    def ticker() -> None:
        started.set()
        while not done.is_set():
            ticks.append(None)
            sleep(0)

    thread = threading.Thread(target=ticker)
    switch_interval = sys.getswitchinterval()
    sys.setswitchinterval(1.0)
    try:
        thread.start()
        assert started.wait(timeout=2)
        ticks.clear()
        result = operation()
        worker_ticks = len(ticks)
    finally:
        done.set()
        sys.setswitchinterval(switch_interval)
        thread.join(timeout=2)

    assert not thread.is_alive()
    assert worker_ticks > 0
    return result


def _render_one(doc: rk.Document, citation: str | rk.Cite | rk.CitationGroup) -> rk.Rendered:
    return doc.render([rk.Citation("citation", citation)])["citation"]


def test_public_document_example_renders_text_html_and_tree() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    style = rk.Style.load("apa")

    doc = rk.Document(library, style, locale="en-US")
    rendered = doc.render(
        [
            rk.Citation("first", "doe2024"),
            rk.Citation(
                "second",
                rk.CitationGroup([rk.Cite("doe2024", locator="12", label="page"), "roe2022"]),
            ),
        ]
    )
    first = rendered["first"]
    second = rendered["second"]
    bibliography = rendered.bibliography
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


def test_document_accepts_named_citation_groups() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    group = rk.CitationGroup(key for key in ["doe2024", "roe2022"])
    rendered = doc.render([rk.Citation("group", group)])

    assert len(group) == 2
    assert [item.key for item in group.items] == ["doe2024", "roe2022"]
    assert rendered["group"].text
    assert rendered.bibliography.text


def test_document_rejects_unnamed_iterable_citation_groups() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    with pytest.raises(TypeError, match="Citation"):
        doc.render(cast(Any, ["doe2024", "roe2022"]))


def test_citation_group_requires_at_least_one_citation() -> None:
    with pytest.raises(ValueError, match="at least one citation"):
        rk.CitationGroup([])


def test_one_off_helpers_render_citation_and_bibliography() -> None:
    citation = rk.cite(FIXTURES / "basic.bib", "doe2024", style="ieee")
    bibliography = rk.full_bibliography(FIXTURES / "basic.bib", style="chicago-author-date")

    assert citation.text
    assert bibliography.text
    assert bibliography.html
    assert "Doe" in bibliography.text
    assert "Roe" in bibliography.text


def test_one_off_cite_accepts_named_citation_groups() -> None:
    rendered = rk.cite(
        FIXTURES / "basic.bib",
        rk.CitationGroup(["doe2024", "roe2022"]),
        style="apa",
    )

    assert "Doe" in rendered.text
    assert "Roe" in rendered.text


def test_one_off_cite_rejects_unnamed_iterable_citation_groups() -> None:
    with pytest.raises(TypeError, match="CitationGroup"):
        rk.cite(
            FIXTURES / "basic.bib",
            cast(Any, ("doe2024", "roe2022")),
            style="apa",
        )


def test_one_off_helpers_accept_loaded_style_objects() -> None:
    style = rk.Style.load("apa")

    citation = rk.cite(FIXTURES / "basic.bib", "doe2024", style=style)
    bibliography = rk.full_bibliography(FIXTURES / "basic.bib", style=style)

    assert "Doe" in citation.text
    assert "Doe" in bibliography.text


def test_real_arxiv_bibtex_fixture_parses_inspects_and_renders() -> None:
    library = rk.Library.read(ARXIV_FIXTURE, recovery="report")
    raw = rk.BibDocument.read(ARXIV_FIXTURE)
    rows = {row["key"]: row for row in library.project(["key", "title", "doi", "volume"])}
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")
    bibliography = doc.full_bibliography()
    one_off = rk.cite(ARXIV_FIXTURE, "Kimi_K2.5", style="apa")

    assert len(library) == 12
    assert library.diagnostics == []
    assert len(raw.entries) == 12
    assert raw.comments[0].startswith("% Real BibTeX subset")
    title = rows["DeepResearchGym"]["title"]
    assert isinstance(title, str)
    assert title.startswith("DeepResearchGym")
    assert rows["DeepResearchGym"]["doi"] == "10.48550/ARXIV.2505.19253"
    assert rows["ijcai2019p684"]["volume"] is None
    assert _render_one(doc, "ijcai2019p684").text == "(Chen et al., 2019)"
    assert one_off.text == "(Team, 2026)"
    assert "Ancient–Modern Chinese Translation" in bibliography.text
    assert "10.48550/ARXIV.2505.19253" in bibliography.text


def test_document_bibliography_scope_is_explicit() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    cited = doc.cited_bibliography([])
    full = doc.full_bibliography()

    assert cited.text == ""
    assert cited.html == ""
    assert "Doe" in full.text
    assert "Roe" in full.text


def test_bibliography_render_releases_gil_for_worker_thread() -> None:
    library = rk.Library.parse_bibtex(_many_bibtex_records(2000))
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    rendered = _run_with_worker_progress(lambda: doc.full_bibliography())

    assert rendered.text


def test_raw_bib_document_write_releases_gil_for_worker_thread(tmp_path: Path) -> None:
    raw = rk.BibDocument.parse(_many_bibtex_records(10_000))
    output = tmp_path / "large.bib"

    _run_with_worker_progress(lambda: raw.write(output))

    assert output.read_text(encoding="utf-8").count("@article") == 10_000


def test_library_parse_accepts_source_strings_and_mapping_helpers() -> None:
    library = rk.Library.parse_bibtex(
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
    assert library.project(["key", "entry_type", "type"]) == [
        {"key": "inline", "entry_type": "Article", "type": "Article"}
    ]
    assert library.project(("key", "title"), keys=("inline",)) == [
        {"key": "inline", "title": "Inline Source"}
    ]

    with pytest.raises(KeyError):
        library.project(["key"], keys=["missing"])
    with pytest.raises(ValueError, match="unsupported projection field"):
        library.project(["unknown"])

    yaml_library = rk.Library.parse_yaml((FIXTURES / "parent.yaml").read_text())
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


def test_mode_and_option_arguments_are_keyword_only() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib", recovery="report")
    parsed = rk.Library.parse_bibtex("@article{inline, title={Inline Source}, year={2024}}")
    style = rk.Style.load("apa")
    doc = rk.Document(library, style, locale="en-US")

    assert parsed.keys() == ["inline"]
    assert doc.full_bibliography().text
    assert library.project(["key"], keys=["doe2024"]) == [{"key": "doe2024"}]
    assert rk.Cite("doe2024", locator="12", label="page").locator == "12"

    with pytest.raises(TypeError):
        cast(Any, rk.Library.read)(FIXTURES / "basic.bib", True)
    with pytest.raises(TypeError):
        cast(Any, rk.Library.parse_bibtex)(
            "@article{inline, title={Inline Source}, year={2024}}",
            True,
        )
    with pytest.raises(TypeError):
        cast(Any, rk.Cite)("doe2024", "12", "page")
    with pytest.raises(TypeError):
        cast(Any, rk.Document)(library, style, "en-US")
    with pytest.raises(TypeError):
        cast(Any, doc.full_bibliography)(True)
    with pytest.raises(TypeError):
        cast(Any, library.project)(["key"], ["doe2024"])


def test_library_values_entry_types_and_parent_lists_are_public_contracts() -> None:
    library = rk.Library.read(FIXTURES / "parent.yaml")
    entry = library["doe2024"]

    assert [value.key for value in library.values()] == ["doe2024"]
    assert entry.entry_type == "Article"
    assert [parent.entry_type for parent in entry.parents] == ["Periodical"]
    assert entry.parents[0].title == "Journal of Citation Systems"


def test_entry_parent_chains_preserve_nested_hayagriva_parents() -> None:
    library = rk.Library.parse_yaml(
        """chapter:
  type: Chapter
  title: Nested Chapter
  parent:
    type: Book
    title: Parent Book
    parent:
      type: Anthology
      title: Grand Collection
"""
    )

    entry = library["chapter"]
    assert entry.parents[0].title == "Parent Book"
    assert entry.parents[0].parents[0].entry_type == "Anthology"


def test_refkit_import_reports_runtime_core_metadata() -> None:
    assert rk.__version__ == metadata.version("refkit")
    assert rk.check_refkit_core_version()
    assert rk.build_info.startswith(f"refkit-core {metadata.version('refkit-core')}")
    assert rk.build_mode in {"debug", "release"}


def test_refkit_import_rejects_mismatched_core_version(monkeypatch: pytest.MonkeyPatch) -> None:
    required_core_version = metadata.version("refkit-core")
    mismatched_core = ModuleType("refkit_core")
    cast(Any, mismatched_core).__version__ = f"{required_core_version}.mismatch"

    monkeypatch.setitem(sys.modules, "refkit_core", mismatched_core)
    monkeypatch.delitem(sys.modules, "refkit")
    monkeypatch.syspath_prepend(str(ROOT / "src"))
    try:
        with pytest.raises(SystemError, match=f"requires {required_core_version}"):
            importlib.import_module("refkit")
    finally:
        sys.modules["refkit"] = rk


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

    rendered = rk.full_bibliography(source, style="apa")

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

    rendered = rk.full_bibliography(source, style="apa")

    assert 'href="javascript:alert(1)"' not in rendered.html
    assert "javascript:alert(1)" in rendered.html
    nodes = list(_tree_nodes(rendered.tree))
    link_nodes = [node for node in nodes if node.get("meta") == "Link"]
    text_nodes = [node for node in nodes if node.get("kind") == "Text"]

    assert all("javascript:alert(1)" not in node.values() for node in link_nodes)
    assert any(node.get("text") == "javascript:alert(1)" for node in text_nodes)


def test_rendered_html_preserves_csl_formatting() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    rendered = doc.cited_bibliography([rk.Citation("citation", "doe2024")])

    assert "<i>Journal of Citation Systems</i>" in rendered.html
    assert '<a href="https://doi.org/10.1234/refkit.2024">' in rendered.html


def test_bibliography_text_and_tree_include_second_field_labels() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("ieee"), locale="en-US")

    rendered = doc.cited_bibliography([rk.Citation("citation", "doe2024")])
    entry = cast(dict[str, Any], rendered.tree[0])
    first_field = cast(dict[str, Any], entry["first_field"])

    assert rendered.text.startswith("[1]")
    assert entry["kind"] == "bibliography-entry"
    assert first_field["kind"] == "Element"
    assert first_field["meta"] == "CitationNumber"
    assert any(node.get("text") == "[1]" for node in _tree_nodes(first_field))


def test_rendered_tree_exposes_documented_structured_keys() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("ieee"), locale="en-US")
    rendered = doc.render([rk.Citation("citation", "doe2024")])
    citation_tree = rendered["citation"].tree
    bibliography_tree = rendered.bibliography.tree

    assert citation_tree[0]["kind"] == "Element"
    assert "children" in citation_tree[0]
    assert bibliography_tree[0]["kind"] == "bibliography-entry"
    assert bibliography_tree[0]["key"] == "doe2024"
    assert bibliography_tree[0]["first_field"] is not None
    assert bibliography_tree[0]["children"][0]["kind"] == "Element"


def test_rendered_tree_uses_stable_public_strings() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("ieee"), locale="en-US")

    citation_tree = _render_one(doc, "doe2024").tree
    entry_node = cast(dict[str, Any], citation_tree[0])
    nodes = list(_tree_nodes(citation_tree))
    citation_number = next(node for node in nodes if node.get("meta") == "CitationNumber")
    formatted_text = next(node for node in nodes if node.get("kind") == "Text")
    formatting = cast(dict[str, Any], formatted_text["formatting"])

    assert entry_node["meta"] == "Entry"
    assert citation_number["meta"] == "CitationNumber"
    assert set(formatting) == {
        "font_style",
        "font_variant",
        "font_weight",
        "text_decoration",
        "vertical_align",
    }
    assert all(isinstance(value, str) for value in formatting.values())


def test_library_reads_yaml_and_selects_parent_periodical() -> None:
    library = rk.Library.read(FIXTURES / "parent.yaml")
    matches = library.select("article > periodical[volume]")

    assert len(matches) == 1
    assert matches[0].key == "doe2024"
    assert matches[0].title == "Refkit for Bibliographies"
    assert matches[0].parents[0].title == "Journal of Citation Systems"


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
    assert rk.Library.parse_yaml(source).keys() == library.keys()


def test_library_reads_bibtex_and_biblatex_sources() -> None:
    source = (FIXTURES / "typst-biblatex.bib").read_text(encoding="utf-8")
    read_library = rk.Library.read(FIXTURES / "typst-biblatex.bib")

    assert read_library["biblatex2023"].title == "The biblatex Package"
    assert read_library["arrgh"].parents[0].title == "Journal of Political Economy"
    assert read_library["arrgh"].volume == "115"
    assert read_library["tolkien54"].parents[0].title == "The Lord of the Rings"

    library = rk.Library.parse_bibtex(source)

    assert library["arrgh"].volume == "115"
    assert library.project(["key", "title", "volume"], keys=["tolkien54"]) == [
        {
            "key": "tolkien54",
            "title": "The Fellowship of the Ring",
            "volume": "1",
        }
    ]


def test_library_reports_unsupported_read_extension(tmp_path: Path) -> None:
    source = tmp_path / "refs.json"
    source.write_text("{}", encoding="utf-8")

    with pytest.raises(rk.RefkitError, match='unsupported bibliography extension "json"'):
        rk.Library.read(source)


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
    assert "Doe" in _render_one(document, "doe2024").text

    with pytest.raises(ValueError, match="unknown bundled style"):
        rk.Style.load("unknown-style")
    with pytest.raises(ValueError, match="invalid CSL XML"):
        rk.Style.from_xml("<style/>")
    with pytest.raises(ValueError, match="unknown bundled locale"):
        rk.Locale.load("zz-ZZ")


def test_library_report_recovery_keeps_valid_bibtex_records_with_diagnostics(
    tmp_path: Path,
) -> None:
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

    library = rk.Library.read(source, recovery="report")

    assert library.keys() == ["valid"]
    assert library["valid"].title == "Kept Entry"
    assert library.diagnostics
    assert "ignored malformed BibTeX block" in library.diagnostics[0]


def test_library_parse_bibtex_default_recovery_raises_on_malformed_records() -> None:
    with pytest.raises(rk.RefkitError, match="parse error"):
        rk.Library.parse_bibtex(
            """@article{valid,
  author = {Doe, Jane},
  title = {Kept Entry},
  year = {2024}
}

@broken{missing,
  title = {No close}
"""
        )


def test_library_parse_bibtex_report_recovery_keeps_valid_records() -> None:
    library = rk.Library.parse_bibtex(
        """@article{valid,
  author = {Doe, Jane},
  title = {Kept Entry},
  year = {2024}
}

@broken{missing,
  title = {No close}
""",
        recovery="report",
    )

    assert library.keys() == ["valid"]


def test_library_rejects_malformed_only_bibtex_without_rejecting_empty_input(
    tmp_path: Path,
) -> None:
    with pytest.raises(rk.RefkitError, match="parse error"):
        rk.Library.parse_bibtex("@broken{missing")

    malformed = tmp_path / "malformed.bib"
    malformed.write_text("@broken{missing")
    with pytest.raises(rk.RefkitError, match="parse error"):
        rk.Library.read(malformed)

    assert rk.Library.parse_bibtex("").is_empty()
    assert rk.Library.parse_bibtex("% only a comment\n").is_empty()


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

    library = rk.Library.read(source, recovery="report")

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

    library = rk.Library.read(source, recovery="report")

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

    library = rk.Library.read(source, recovery="report")

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

    library = rk.Library.read(source, recovery="report")

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

    library = rk.Library.read(source, recovery="report")

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

    library = rk.Library.read(source, recovery="report")

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

    library = rk.Library.read(source, recovery="report")

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

    library = rk.Library.read(source, recovery="report")

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

    library = rk.Library.read(source, recovery="report")

    assert library.keys() == ["valid"]
    assert len(library.diagnostics) == 3
    assert all("ignored malformed BibTeX block" in item for item in library.diagnostics)


def test_library_recovery_ignores_invalid_typed_fields() -> None:
    library = rk.Library.parse_bibtex(
        """@article{badmonth,
  author = {Doe, Jane},
  title = {Bad Month},
  year = {2024},
  month = {16}
}
""",
        recovery="report",
    )

    assert library.keys() == ["badmonth"]
    assert library["badmonth"].title == "Bad Month"
    assert 'ignored BibTeX field "month"' in library.diagnostics[0]


def test_library_recovery_literalizes_unknown_bibtex_abbreviations() -> None:
    library = rk.Library.parse_bibtex(
        """@article{macro,
  author = {Doe, Jane},
  title = {Macro Journal},
  year = {2024},
  journal = JMLR # { Extra}
}
""",
        recovery="report",
    )

    assert library.keys() == ["macro"]
    assert library["macro"].title == "Macro Journal"
    assert "unknown abbreviation" in library.diagnostics[0]


def test_library_read_decodes_windows_1252_bibtex(tmp_path: Path) -> None:
    source = tmp_path / "windows-1252.bib"
    source.write_bytes(
        b"""@article{encoded,
  author = {Doe, Jane},
  title = {Smart \x92 Quote},
  year = {2024}
}
"""
    )

    raw = rk.BibDocument.read(source)
    library = rk.Library.read(source, recovery="report")

    assert raw.entries["encoded"].fields["title"].value == "Smart ’ Quote"
    assert library["encoded"].title == "Smart ’ Quote"
    assert "decoded" in library.diagnostics[0]


def test_missing_reference_raises_structured_error() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    with pytest.raises(rk.MissingReferenceError, match="missing-key"):
        doc.render([rk.Citation("missing", "missing-key")])

    assert "Doe" in _render_one(doc, "doe2024").text


def test_document_render_rejects_duplicate_citation_ids() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    with pytest.raises(ValueError, match='duplicate citation id "same"'):
        doc.render(
            [
                rk.Citation("same", "doe2024"),
                rk.Citation("same", "smith2023"),
            ]
        )


def test_invalid_locator_label_raises_value_error() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    with pytest.raises(ValueError, match='unknown locator label "nonsense"'):
        doc.render([rk.Citation("citation", rk.Cite("doe2024", locator="12", label="nonsense"))])


def test_valid_locator_label_renders_locator_text() -> None:
    library = rk.Library.read(FIXTURES / "basic.bib")
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    rendered = _render_one(doc, rk.Cite("doe2024", locator="12", label="page"))

    assert rendered.text == "(Doe, 2024, p. 12)"


def test_ambiguous_author_date_citations_fall_back_to_disambiguation() -> None:
    library = rk.Library.parse_bibtex(
        """@article{first,
  author = {Doe, Jane},
  title = {First},
  journal = {Journal},
  year = {2024}
}

@article{second,
  author = {Doe, Jane},
  title = {Second},
  journal = {Journal},
  year = {2024}
}
""",
    )
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    rendered = doc.render(
        [
            rk.Citation("first", "first"),
            rk.Citation("second", "second"),
        ]
    )

    assert rendered["first"].text == "(Doe, 2024a)"
    assert rendered["second"].text == "(Doe, 2024b)"
    assert "2024a" in rendered.bibliography.text


def test_bibliography_only_year_suffix_does_not_change_citation_text() -> None:
    style = rk.Style.from_xml(
        """<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info>
    <title>Bibliography Suffix Test</title>
    <id>https://example.com/bibliography-suffix-test</id>
    <updated>2024-01-01T00:00:00+00:00</updated>
  </info>
  <citation disambiguate-add-year-suffix="true">
    <layout prefix="(" suffix=")">
      <names variable="author"/>
      <date variable="issued" prefix=", ">
        <date-part name="year"/>
      </date>
    </layout>
  </citation>
  <bibliography>
    <layout>
      <names variable="author"/>
      <date variable="issued" prefix=" (" suffix=")">
        <date-part name="year"/>
      </date>
      <text variable="year-suffix"/>
      <text variable="title" prefix=". "/>
    </layout>
  </bibliography>
</style>
""",
    )
    library = rk.Library.parse_bibtex(
        """@article{first,
  author = {Doe, Jane},
  title = {First},
  journal = {Journal},
  year = {2024}
}

@article{second,
  author = {Doe, Jane},
  title = {Second},
  journal = {Journal},
  year = {2024}
}
""",
    )
    doc = rk.Document(library, style, locale="en-US")

    rendered = doc.render(
        [
            rk.Citation("first", "first"),
            rk.Citation("second", "second"),
        ]
    )
    first = rendered["first"].text
    second = rendered["second"].text
    bibliography = rendered.bibliography.text

    assert first == "(Jane Doe, 2024)"
    assert second == "(Jane Doe, 2024)"
    assert "(2024)a" in bibliography
    assert "(2024)b" in bibliography


def test_whole_document_render_resolves_disambiguation_before_returning_citations() -> None:
    library = rk.Library.parse_bibtex(
        """@article{first,
  author = {Doe, Jane},
  title = {First},
  journal = {Journal},
  year = {2024}
}

@article{second,
  author = {Doe, Jane},
  title = {Second},
  journal = {Journal},
  year = {2024}
}
""",
    )
    doc = rk.Document(library, rk.Style.load("apa"), locale="en-US")

    rendered = doc.render(
        [
            rk.Citation("first", rk.Cite("first", locator="12", label="page")),
            rk.Citation("second", "second"),
        ]
    )

    assert rendered["first"].text == "(Doe, 2024a, p. 12)"
    assert rendered["second"].text == "(Doe, 2024b)"


def test_repeated_key_respects_subsequent_name_rules() -> None:
    style = rk.Style.from_xml(
        """<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info>
    <title>Subsequent Names Test</title>
    <id>https://example.com/subsequent-names-test</id>
    <updated>2024-01-01T00:00:00+00:00</updated>
  </info>
  <citation
    et-al-min="10"
    et-al-use-first="10"
    et-al-subsequent-min="3"
    et-al-subsequent-use-first="1"
  >
    <layout prefix="(" suffix=")">
      <names variable="author">
        <name and="text" delimiter=", "/>
        <et-al term="et-al"/>
      </names>
      <date variable="issued" prefix=", ">
        <date-part name="year"/>
      </date>
    </layout>
  </citation>
  <bibliography>
    <layout>
      <text variable="title"/>
    </layout>
  </bibliography>
</style>
""",
    )
    library = rk.Library.parse_yaml(
        """team:
  type: Article
  author: ["Alpha, Ann", "Beta, Bob", "Gamma, Gus"]
  title: Team Work
  date: 2024
  parent:
    type: Periodical
    title: Journal
"""
    )
    doc = rk.Document(library, style, locale="en-US")

    rendered = doc.render(
        [
            rk.Citation("first", "team"),
            rk.Citation("second", "team"),
        ]
    )
    first = rendered["first"].text
    second = rendered["second"].text

    assert first != second
    assert "et al" in second


def test_names_substitute_citation_number_uses_bibliography_sort() -> None:
    style = rk.Style.from_xml(
        """<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info>
    <title>Substitute Citation Number Test</title>
    <id>https://example.com/substitute-citation-number-test</id>
    <updated>2024-01-01T00:00:00+00:00</updated>
  </info>
  <citation>
    <layout prefix="[" suffix="]">
      <names variable="author">
        <substitute>
          <number variable="citation-number"/>
        </substitute>
      </names>
    </layout>
  </citation>
  <bibliography>
    <sort>
      <key variable="title"/>
    </sort>
    <layout>
      <text variable="title"/>
    </layout>
  </bibliography>
</style>
""",
    )
    library = rk.Library.parse_yaml(
        """a:
  type: Article
  title: Alpha
  date: 2024
b:
  type: Article
  title: Beta
  date: 2024
"""
    )
    doc = rk.Document(library, style, locale="en-US")

    rendered = doc.render(
        [
            rk.Citation("b", "b"),
            rk.Citation("a", "a"),
        ]
    )

    assert rendered["b"].text == "[2]"
    assert rendered["a"].text == "[1]"


def test_names_substitute_position_condition_uses_full_history() -> None:
    style = rk.Style.from_xml(
        """<style xmlns="http://purl.org/net/xbiblio/csl" version="1.0" class="in-text">
  <info>
    <title>Substitute Position Test</title>
    <id>https://example.com/substitute-position-test</id>
    <updated>2024-01-01T00:00:00+00:00</updated>
  </info>
  <citation>
    <layout prefix="(" suffix=")">
      <names variable="author">
        <substitute>
          <choose>
            <if position="first">
              <text value="first"/>
            </if>
            <else>
              <text value="later"/>
            </else>
          </choose>
        </substitute>
      </names>
    </layout>
  </citation>
  <bibliography>
    <layout>
      <text variable="title"/>
    </layout>
  </bibliography>
</style>
""",
    )
    library = rk.Library.parse_yaml(
        """item:
  type: Article
  title: Position Work
  date: 2024
"""
    )
    doc = rk.Document(library, style, locale="en-US")

    rendered = doc.render([rk.Citation("item", "item")])

    assert rendered["item"].text == "(first)"


def test_raw_bib_document_blocks_preserve_source_order_and_spans() -> None:
    source_path = FIXTURES / "raw-duplicates.bib"
    source = source_path.read_text(encoding="utf-8")
    raw = rk.BibDocument.read(source_path)
    blocks = raw.blocks
    spans = [tuple(block["span"]) for block in blocks]

    assert spans == sorted(spans)
    assert all(start < end for start, end in spans)
    assert all(left[1] <= right[0] for left, right in zip(spans, spans[1:], strict=False))
    assert {block["kind"] for block in blocks} == {
        "comment",
        "preamble",
        "string",
        "whitespace",
        "other",
        "entry",
        "failed",
    }
    for block in blocks:
        start, end = block["span"]
        if block["kind"] in {"comment", "failed", "other"}:
            raw_block = cast(dict[str, Any], block)
            assert raw_block["raw"] == source[start:end]


def test_raw_bib_document_duplicate_entries_require_explicit_get_all() -> None:
    raw = rk.BibDocument.read(FIXTURES / "raw-duplicates.bib")

    assert raw.entries.unique_keys() == ["dup", "later"]
    assert [entry.key for entry in raw.entries.occurrences()] == ["dup", "dup", "later"]
    assert [entry.key for entry in raw.entries.get_all("dup")] == ["dup", "dup"]
    assert raw.entries.get_all("missing") == []
    assert raw.entries.get_unique("missing") is None

    with pytest.raises(rk.RefkitError, match='entry key "dup" is ambiguous'):
        raw.entries["dup"]
    with pytest.raises(rk.RefkitError, match="use entries.get_all"):
        raw.entries.get_unique("dup")

    later = raw.entries["later"]
    assert later.key == "later"
    assert later.fields["title"].value == "Later Entry"


def test_raw_bib_document_duplicate_fields_require_explicit_get_all() -> None:
    raw = rk.BibDocument.read(FIXTURES / "raw-duplicates.bib")
    entry = raw.entries.get_all("dup")[0]

    assert entry.fields.unique_keys() == ["title", "journal", "year"]
    assert [field.name for field in entry.fields.occurrences()] == [
        "title",
        "title",
        "journal",
        "year",
    ]
    assert [field.value for field in entry.fields.get_all("title")] == [
        "First Title",
        "Second Title",
    ]
    assert entry.fields.get_all("missing") == []
    assert entry.fields.get_unique("missing") is None

    with pytest.raises(rk.RefkitError, match='field "title" in entry "dup" is ambiguous'):
        entry.fields["title"]
    with pytest.raises(rk.RefkitError, match="use fields.get_all"):
        entry.fields.get_unique("title")

    assert entry.fields["journal"].value == "j"


def test_raw_bib_document_edits_duplicate_occurrences_without_losing_raw_blocks(
    tmp_path: Path,
) -> None:
    raw = rk.BibDocument.read(FIXTURES / "raw-duplicates.bib")
    first_dup, second_dup = raw.entries.get_all("dup")
    first_dup.fields.get_all("title")[1].value = "Corrected Second Field"
    second_dup.fields["title"].value = "Corrected Duplicate Entry"
    output = tmp_path / "updated-duplicates.bib"

    raw.write(output)

    text = output.read_text(encoding="utf-8")
    assert "% duplicate raw fixture" in text
    assert '@preamble{"Duplicate fixture"}' in text
    assert "@string{j = {Journal of Duplicate Contracts}}" in text
    assert "raw prose outside entries" in text
    assert "@broken{bad" in text
    assert "Corrected Second Field" in text
    assert "Corrected Duplicate Entry" in text

    written = rk.BibDocument.read(output)
    written_first, written_second = written.entries.get_all("dup")
    assert [field.value for field in written_first.fields.get_all("title")] == [
        "First Title",
        "Corrected Second Field",
    ]
    assert written_second.fields["title"].value == "Corrected Duplicate Entry"
    assert written.entries["later"].fields["title"].value == "Later Entry"
    assert written.failed_blocks[0]["raw"].startswith("@broken{bad")


def test_raw_bib_document_preserves_blocks_and_writes_field_edit(tmp_path: Path) -> None:
    raw = rk.BibDocument.read(FIXTURES / "raw.bib")
    blocks = raw.blocks
    source = (FIXTURES / "raw.bib").read_text(encoding="utf-8")

    assert raw.comments[0].startswith("% library comment")
    assert raw.preamble == "BibTeX preamble"
    assert raw.strings["jcs"] == "Journal of Citation Systems"
    assert blocks[0]["kind"] == "comment"
    assert blocks[0]["raw"] == "% library comment\n"
    assert blocks[1]["kind"] == "preamble"
    preamble_start, preamble_end = blocks[1]["span"]
    assert source[preamble_start:preamble_end] == '@preamble{"BibTeX preamble"}'
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
    assert "journal = jcs" in text
    written = rk.BibDocument.read(output)
    assert written.entries["doe2024"].fields["title"].value == "Corrected title"
    assert written.entries["doe2024"].fields["journal"].value == "jcs"


def test_raw_bib_document_preserves_typst_biblatex_blocks(tmp_path: Path) -> None:
    raw = rk.BibDocument.read(FIXTURES / "typst-raw.bib")

    assert raw.comments[0].startswith("@comment")
    assert any(comment.startswith("% Comments before") for comment in raw.comments)
    assert raw.strings["benchjournal"] == "Journal of Citation Benchmarks"
    assert raw.preamble == '"Reference " # "fixture"'
    assert raw.entries.unique_keys() == ["fischer2022equivalence", "roes2003belief"]
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
    assert "@inproceedings{conigliocorbalan" in text
    assert "author    {Marcelo Coniglio and Maria Corbalan}" in text
    written = rk.BibDocument.read(output)
    assert written.entries["roes2003belief"].fields["title"].value == "Edited belief title"


def test_raw_bib_document_parse_accepts_source_strings_and_mapping_helpers(
    tmp_path: Path,
) -> None:
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
    assert "inline" in raw.entries
    assert "missing" not in raw.entries
    entry = raw.entries.get_unique("inline")
    assert entry is not None
    assert entry.key == "inline"
    assert raw.entries.get_unique("missing") is None
    assert [entry.key for entry in raw.entries.occurrences()] == ["inline"]
    assert [entry.key for entry in raw.entries.get_all("inline")] == ["inline"]
    assert raw.entries["inline"].fields
    assert not raw.entries["inline"].fields.is_empty()
    title = raw.entries["inline"].fields.get_unique("title")
    assert title is not None
    assert title.name == "title"
    assert title.value == "Inline Raw"
    assert [field.name for field in raw.entries["inline"].fields.occurrences()] == ["title"]
    assert [field.value for field in raw.entries["inline"].fields.get_all("title")] == [
        "Inline Raw"
    ]
    assert "title" in raw.entries["inline"].fields
    assert "missing" not in raw.entries["inline"].fields
    assert raw.entries["inline"].fields.get_unique("missing") is None
    assert raw.failed_blocks

    output = tmp_path / "inline.bib"
    raw.write(output)

    assert "@article{inline" in output.read_text(encoding="utf-8")


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
    raw = rk.BibDocument.read(FIXTURES / "raw.bib")
    entry = raw.entries["doe2024"]
    field = entry.fields["title"]

    assert isinstance(raw.entries, rk.BibEntryMap)
    assert isinstance(entry, rk.BibEntry)
    assert isinstance(entry.fields, rk.BibFieldMap)
    assert isinstance(field, rk.BibField)


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
    written = rk.BibDocument.read(output)
    assert written.entries["concat"].fields["title"].value == "New"


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
    assert [entry.key for entry in raw.entries.occurrences()] == ["same", "same"]
    duplicates = raw.entries.get_all("same")
    assert [entry.fields["title"].value for entry in duplicates] == ["First", "Second"]

    with pytest.raises(rk.RefkitError, match='entry key "same" is ambiguous'):
        raw.entries["same"]
    with pytest.raises(rk.RefkitError, match='entry key "same" is ambiguous'):
        raw.entries.get_unique("same")

    duplicates[1].fields["title"].value = "Corrected second"
    output = tmp_path / "duplicates-out.bib"
    raw.write(output)

    text = output.read_text()
    assert "title = {First}" in text
    assert "title = {Corrected second}" in text
    assert text.count("@article{same") == 2

    recovered = rk.Library.read(output, recovery="report")
    assert recovered["same"].title == "First"
    assert 'ignored duplicate BibTeX entry key "same"' in recovered.diagnostics[0]


def test_raw_bib_document_duplicate_fields_are_addressable_by_occurrence(tmp_path: Path) -> None:
    raw = rk.BibDocument.parse(
        """@article{duplicate,
  title = {First},
  TITLE = {Second},
  year = {2024}
}
"""
    )

    entry = raw.entries["duplicate"]
    assert entry.fields.unique_keys() == ["title", "year"]
    assert [field.name for field in entry.fields.occurrences()] == ["title", "title", "year"]
    titles = entry.fields.get_all("title")
    assert [field.value for field in titles] == ["First", "Second"]

    with pytest.raises(rk.RefkitError, match='field "title" in entry "duplicate" is ambiguous'):
        entry.fields["title"]
    with pytest.raises(rk.RefkitError, match='field "title" in entry "duplicate" is ambiguous'):
        entry.fields.get_unique("title")

    titles[0].value = "Corrected first"
    output = tmp_path / "duplicate-fields-out.bib"
    raw.write(output)

    text = output.read_text()
    assert "title = {Corrected first}" in text
    assert "TITLE = {Second}" in text
    assert "year = {2024}" in text


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


def test_raw_bib_document_keeps_percent_encoded_urls_inside_braced_values(
    tmp_path: Path,
) -> None:
    source = tmp_path / "percent-url.bib"
    source.write_text(
        """@article{encoded,
  title = {Percent Encoded URL},
  url = {https://example.test/path%2Fpaper?partnerID=40&md5=abc},
  year = {2024}
}
""",
    )

    raw = rk.BibDocument.read(source)

    assert raw.failed_blocks == []
    assert raw.entries["encoded"].fields["url"].value == (
        "https://example.test/path%2Fpaper?partnerID=40&md5=abc"
    )

    output = tmp_path / "percent-url-out.bib"
    raw.entries["encoded"].fields["title"].value = "Edited title"
    raw.write(output)

    written = rk.BibDocument.read(output)
    library = rk.Library.read(output)
    assert written.failed_blocks == []
    assert written.entries["encoded"].fields["url"].value == (
        "https://example.test/path%2Fpaper?partnerID=40&md5=abc"
    )
    assert library["encoded"].title == "Edited title"


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
