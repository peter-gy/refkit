from __future__ import annotations

from collections.abc import Callable, Iterable, Mapping
from dataclasses import dataclass, field
from pathlib import Path
from time import perf_counter_ns
from typing import Any, cast

from benchmark.fixtures import Workload

OutcomeValue = object


class UnsupportedOperation(Exception):
    def __init__(self, reason: str) -> None:
        super().__init__(reason)
        self.reason = reason


@dataclass(frozen=True)
class OperationOutcome:
    value: OutcomeValue
    count: int
    detail: str = ""


@dataclass(frozen=True)
class PreparedOperation:
    phase: str
    operation: Callable[[], OperationOutcome]
    check: Callable[[OperationOutcome], None]
    metadata: dict[str, object] = field(default_factory=dict)


class PackageAdapter:
    name: str
    distribution: str

    def prepare(self, case: str, workload: Workload, directory: Path) -> PreparedOperation:
        method_name = f"prepare_{case}"
        method = getattr(self, method_name, None)
        if method is None:
            raise UnsupportedOperation(f"{self.name} does not support {case}")
        return method(workload, directory)


class RefkitAdapter(PackageAdapter):
    name = "refkit"
    distribution = "refkit"

    def prepare_bibtex_parse(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        def operation() -> OperationOutcome:
            library = rk.Library.read(workload.bibtex_path)
            return OperationOutcome(library, len(library))

        return _prepared("parse", operation, _count_is(len(workload.records)), setup_included=True)

    def prepare_bibtex_recovery_parse(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        def operation() -> OperationOutcome:
            library = rk.Library.read(workload.dirty_bibtex_path, diagnostics=True)
            return OperationOutcome(
                library, len(library), f"diagnostics={len(library.diagnostics)}"
            )

        return _prepared(
            "parse-recovery",
            operation,
            _count_is(len(workload.records)),
            source_format="dirty_bibtex",
            setup_included=True,
        )

    def prepare_raw_bibtex_parse(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        def operation() -> OperationOutcome:
            document = rk.BibDocument.parse(workload.raw_bibtex)
            return OperationOutcome(document, len(document.entries))

        return _prepared(
            "raw-parse",
            operation,
            _count_is(len(workload.records)),
            source_format="raw_bibtex",
            setup_included=True,
        )

    def prepare_raw_bibtex_write(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        document = rk.BibDocument.parse(workload.raw_bibtex)
        field = document.entries[workload.keys[0]].fields["title"]

        def operation() -> OperationOutcome:
            field.value = "Edited Benchmark Title"
            path = directory / f"refkit-raw-write-{perf_counter_ns()}.bib"
            document.write(path)
            return OperationOutcome(path, len(document.entries), path.name)

        return _prepared(
            "raw-write",
            operation,
            _raw_roundtrip_check(workload.keys),
            source_format="raw_bibtex",
        )

    def prepare_raw_bibtex_roundtrip(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        def operation() -> OperationOutcome:
            document = rk.BibDocument.parse(workload.raw_bibtex)
            field = document.entries[workload.keys[0]].fields["title"]
            field.value = "Edited Benchmark Title"
            path = directory / f"refkit-raw-{perf_counter_ns()}.bib"
            document.write(path)
            return OperationOutcome(path, len(document.entries), path.name)

        return _prepared(
            "raw-write",
            operation,
            _raw_roundtrip_check(workload.keys),
            source_format="raw_bibtex",
            setup_included=True,
        )

    def prepare_style_load(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        def operation() -> OperationOutcome:
            style = rk.Style.load("apa")
            return OperationOutcome(style, 1)

        return _prepared("style-load", operation, _count_is(1), source_format="none")

    def prepare_processor_setup(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse(workload.bibtex)
        style = rk.Style.load("apa")

        def operation() -> OperationOutcome:
            document = rk.Document(library, style, locale="en-US")
            return OperationOutcome(document, 1)

        return _prepared("processor-setup", operation, _count_is(1))

    def prepare_citation_render(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse(workload.bibtex)
        style = rk.Style.load("apa")
        key = workload.keys[0]

        def operation() -> OperationOutcome:
            document = rk.Document(library, style, locale="en-US")
            rendered = document.cite(key)
            return OperationOutcome(rendered.text, 1)

        return _prepared(
            "render",
            operation,
            _all_checks(_count_is(1), _citation_output_matches(workload.records[:1])),
            citation_count=1,
        )

    def prepare_bibliography_render(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse(workload.bibtex)
        style = rk.Style.load("apa")

        def operation() -> OperationOutcome:
            document = rk.Document(library, style, locale="en-US")
            rendered = document.bibliography(all=True)
            return OperationOutcome(rendered.text, len(workload.records))

        return _prepared(
            "render",
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibliography_output_matches(workload.records),
            ),
            citation_count=0,
        )

    def prepare_bibliography_seen_render(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse(workload.bibtex)
        style = rk.Style.load("apa")
        keys = workload.keys

        def operation() -> OperationOutcome:
            document = rk.Document(library, style, locale="en-US")
            for key in keys:
                document.cite(key)
            rendered = document.bibliography()
            return OperationOutcome(rendered.text, len(keys))

        return _prepared(
            "render-bibliography-seen",
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibliography_output_matches(workload.records),
            ),
            citation_count=len(workload.records),
        )

    def prepare_repeated_render(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse(workload.bibtex)
        style = rk.Style.load("apa")
        keys = workload.keys

        def operation() -> OperationOutcome:
            document = rk.Document(library, style, locale="en-US")
            texts = [document.cite(key).text for key in keys]
            return OperationOutcome("\n".join(texts), len(texts))

        return _prepared(
            "steady-render",
            operation,
            _all_checks(
                _count_is(len(keys)),
                _citation_output_matches(workload.records[: len(keys)]),
            ),
            citation_count=len(keys),
        )

    def prepare_rendered_text_access(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        rendered = self._prepared_rendered_citation(workload)

        def operation() -> OperationOutcome:
            text = rendered.text
            return OperationOutcome(text, 1)

        return _prepared(
            "render-output-text",
            operation,
            _all_checks(_count_is(1), _citation_output_matches(workload.records[:1])),
        )

    def prepare_rendered_html_access(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        rendered = self._prepared_rendered_citation(workload)

        def operation() -> OperationOutcome:
            html = rendered.html
            return OperationOutcome(html, 1)

        return _prepared(
            "render-output-html", operation, _text_contains(workload.records[0].family)
        )

    def prepare_rendered_tree_access(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        rendered = self._prepared_rendered_citation(workload)

        def operation() -> OperationOutcome:
            tree = rendered.tree
            return OperationOutcome(tree, len(tree))

        return _prepared("render-output-tree", operation, _count_at_least(1))

    def _prepared_rendered_citation(self, workload: Workload) -> Any:
        import refkit as rk

        library = rk.Library.parse(workload.bibtex)
        style = rk.Style.load("apa")
        document = rk.Document(library, style, locale="en-US")
        return document.cite(workload.keys[0])

    def prepare_one_off_cite(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        key = workload.keys[0]

        def operation() -> OperationOutcome:
            rendered = rk.cite(workload.bibtex_path, key, style="apa")
            return OperationOutcome(rendered.text, 1)

        return _prepared(
            "one-off-render",
            operation,
            _all_checks(_count_is(1), _citation_output_matches(workload.records[:1])),
            setup_included=True,
            citation_count=1,
        )

    def prepare_one_off_bibliography(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        def operation() -> OperationOutcome:
            rendered = rk.bibliography(workload.bibtex_path, style="apa")
            return OperationOutcome(rendered.text, len(workload.records))

        return _prepared(
            "one-off-render",
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibliography_output_matches(workload.records),
            ),
            setup_included=True,
            citation_count=0,
        )

    def prepare_missing_reference(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse(workload.bibtex)
        style = rk.Style.load("apa")

        def operation() -> OperationOutcome:
            document = rk.Document(library, style, locale="en-US")
            try:
                document.cite("missing-reference")
            except rk.MissingReferenceError as exc:
                return OperationOutcome(str(exc), 1, "raised")
            raise AssertionError("missing reference did not raise")  # pragma: no cover

        return _prepared(
            "error",
            operation,
            _all_checks(_count_is(1), _text_contains("missing-reference")),
            citation_count=1,
        )

    def prepare_bulk_materialization(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse(workload.bibtex)

        def operation() -> OperationOutcome:
            rows = library.project(["key", "title"])
            return OperationOutcome(rows, len(rows))

        return _prepared(
            "materialize",
            operation,
            _projection_contains(workload.records, required_fields=("key", "title")),
        )

    def prepare_library_keys(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse(workload.bibtex)

        def operation() -> OperationOutcome:
            keys = library.keys()
            return OperationOutcome(keys, len(keys))

        return _prepared("inspect", operation, _keys_are(workload.keys))

    def prepare_entry_lookup(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse(workload.bibtex)
        keys = _lookup_keys(workload)

        def operation() -> OperationOutcome:
            rows = library.get_many(keys)
            return OperationOutcome(rows, len(rows))

        return _prepared("inspect", operation, _entries_match(workload.records[: len(keys)]))

    def prepare_field_projection(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse(workload.bibtex)

        def operation() -> OperationOutcome:
            rows = library.project(["key", "title", "doi", "volume"])
            return OperationOutcome(rows, len(rows))

        return _prepared(
            "inspect",
            operation,
            _projection_contains(
                workload.records,
                required_fields=("key", "title", "doi", "volume"),
            ),
        )


class CiteprocPyAdapter(PackageAdapter):
    name = "citeproc-py"
    distribution = "citeproc-py"

    def prepare_bibtex_parse(self, workload: Workload, directory: Path) -> PreparedOperation:
        from citeproc.source.bibtex import BibTeX

        def operation() -> OperationOutcome:
            source = BibTeX(str(workload.bibtex_path), encoding="utf-8")
            return OperationOutcome(source, len(list(source.keys())))

        return _prepared("parse", operation, _count_is(len(workload.records)), setup_included=True)

    def prepare_bibtex_recovery_parse(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        from citeproc.source.bibtex import BibTeX

        def operation() -> OperationOutcome:
            try:
                source = BibTeX(str(workload.dirty_bibtex_path), encoding="utf-8")
            except Exception as exc:
                return OperationOutcome("", 0, _error_detail(exc))
            return OperationOutcome(source, len(list(source.keys())))

        return _prepared(
            "parse-recovery",
            operation,
            _recovery_parse_result(len(workload.records)),
            source_format="dirty_bibtex",
            setup_included=True,
        )

    def prepare_style_load(self, workload: Workload, directory: Path) -> PreparedOperation:
        from citeproc import CitationStylesStyle

        def operation() -> OperationOutcome:
            style = CitationStylesStyle("apa", validate=False)
            return OperationOutcome(style, 1)

        return _prepared("style-load", operation, _count_is(1), source_format="none")

    def prepare_processor_setup(self, workload: Workload, directory: Path) -> PreparedOperation:
        source, style = _citeproc_source_and_style(workload)

        def operation() -> OperationOutcome:
            bibliography = _citeproc_processor(source, style)
            return OperationOutcome(bibliography, 1)

        return _prepared("processor-setup", operation, _count_is(1), source_format="csl_json")

    def prepare_citation_render(self, workload: Workload, directory: Path) -> PreparedOperation:
        source, style = _citeproc_source_and_style(workload)
        citation = _citeproc_citation(workload.keys[:1])

        def operation() -> OperationOutcome:
            bibliography = _citeproc_processor(source, style)
            bibliography.register(citation)
            rendered = bibliography.cite(citation, lambda item: None)
            return OperationOutcome(str(rendered), 1)

        return _prepared(
            "render",
            operation,
            _all_checks(_count_is(1), _citation_output_matches(workload.records[:1])),
            source_format="csl_json",
            citation_count=1,
        )

    def prepare_bibliography_render(self, workload: Workload, directory: Path) -> PreparedOperation:
        source, style = _citeproc_source_and_style(workload)
        citation = _citeproc_citation(workload.keys)

        def operation() -> OperationOutcome:
            bibliography = _citeproc_processor(source, style)
            bibliography.register(citation)
            bibliography.sort()
            rows = [str(item) for item in bibliography.bibliography()]
            return OperationOutcome("\n".join(rows), len(rows))

        return _prepared(
            "render",
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibliography_output_matches(workload.records),
            ),
            source_format="csl_json",
            citation_count=len(workload.records),
        )

    def prepare_bibliography_seen_render(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        source, style = _citeproc_source_and_style(workload)
        citations = [_citeproc_citation([key]) for key in workload.keys]

        def operation() -> OperationOutcome:
            bibliography = _citeproc_processor(source, style)
            for citation in citations:
                bibliography.register(citation)
                bibliography.cite(citation, lambda item: None)
            bibliography.sort()
            rows = [str(item) for item in bibliography.bibliography()]
            return OperationOutcome("\n".join(rows), len(rows))

        return _prepared(
            "render-bibliography-seen",
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibliography_output_matches(workload.records),
            ),
            source_format="csl_json",
            citation_count=len(workload.records),
        )

    def prepare_repeated_render(self, workload: Workload, directory: Path) -> PreparedOperation:
        source, style = _citeproc_source_and_style(workload)
        citations = [_citeproc_citation([key]) for key in workload.keys]

        def operation() -> OperationOutcome:
            bibliography = _citeproc_processor(source, style)
            for citation in citations:
                bibliography.register(citation)
            rendered = [
                str(bibliography.cite(citation, lambda item: None)) for citation in citations
            ]
            return OperationOutcome("\n".join(rendered), len(rendered))

        return _prepared(
            "steady-render",
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _citation_output_matches(workload.records),
            ),
            source_format="csl_json",
            citation_count=len(workload.records),
        )

    def prepare_one_off_cite(self, workload: Workload, directory: Path) -> PreparedOperation:
        key = workload.keys[0]

        def operation() -> OperationOutcome:
            from citeproc import CitationStylesStyle
            from citeproc.source.bibtex import BibTeX

            source = BibTeX(str(workload.bibtex_path), encoding="utf-8")
            style = CitationStylesStyle("apa", validate=False)
            bibliography = _citeproc_processor(source, style)
            citation = _citeproc_citation([key])
            bibliography.register(citation)
            rendered = bibliography.cite(citation, lambda item: None)
            return OperationOutcome(str(rendered), 1)

        return _prepared(
            "one-off-render",
            operation,
            _all_checks(_count_is(1), _citation_output_matches(workload.records[:1])),
            setup_included=True,
            citation_count=1,
        )

    def prepare_one_off_bibliography(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        def operation() -> OperationOutcome:
            from citeproc import CitationStylesStyle
            from citeproc.source.bibtex import BibTeX

            source = BibTeX(str(workload.bibtex_path), encoding="utf-8")
            style = CitationStylesStyle("apa", validate=False)
            bibliography = _citeproc_processor(source, style)
            citation = _citeproc_citation(workload.keys)
            bibliography.register(citation)
            bibliography.sort()
            rows = [str(item) for item in bibliography.bibliography()]
            return OperationOutcome("\n".join(rows), len(rows))

        return _prepared(
            "one-off-render",
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibliography_output_matches(workload.records),
            ),
            setup_included=True,
            citation_count=len(workload.records),
        )

    def prepare_missing_reference(self, workload: Workload, directory: Path) -> PreparedOperation:
        source, style = _citeproc_source_and_style(workload)
        citation = _citeproc_citation(["missing-reference"])

        def operation() -> OperationOutcome:
            bibliography = _citeproc_processor(source, style)
            bibliography.register(citation)
            missing: list[str] = []
            rendered = bibliography.cite(citation, lambda item: missing.append(item.key))
            return OperationOutcome(str(rendered), len(missing), ",".join(missing))

        return _prepared(
            "error",
            operation,
            _all_checks(_count_is(1), _detail_contains("missing-reference")),
            source_format="csl_json",
            citation_count=1,
        )

    def prepare_bulk_materialization(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        from citeproc.source.bibtex import BibTeX

        source = BibTeX(str(workload.bibtex_path), encoding="utf-8")

        def operation() -> OperationOutcome:
            rows = [
                {"key": key, "title": _first(dict(reference).get("title"))}
                for key, reference in source.items()
            ]
            return OperationOutcome(rows, len(rows))

        return _prepared(
            "materialize",
            operation,
            _projection_contains(workload.records, required_fields=("key", "title")),
        )

    def prepare_library_keys(self, workload: Workload, directory: Path) -> PreparedOperation:
        from citeproc.source.bibtex import BibTeX

        source = BibTeX(str(workload.bibtex_path), encoding="utf-8")

        def operation() -> OperationOutcome:
            keys = list(source.keys())
            return OperationOutcome(keys, len(keys))

        return _prepared("inspect", operation, _keys_are(workload.keys))

    def prepare_entry_lookup(self, workload: Workload, directory: Path) -> PreparedOperation:
        from citeproc.source.bibtex import BibTeX

        source = BibTeX(str(workload.bibtex_path), encoding="utf-8")
        keys = _lookup_keys(workload)

        def operation() -> OperationOutcome:
            rows = [source[key] for key in keys]
            return OperationOutcome(rows, len(rows))

        return _prepared("inspect", operation, _entries_match(workload.records[: len(keys)]))

    def prepare_field_projection(self, workload: Workload, directory: Path) -> PreparedOperation:
        from citeproc.source.bibtex import BibTeX

        source = BibTeX(str(workload.bibtex_path), encoding="utf-8")

        def operation() -> OperationOutcome:
            rows = []
            for key, reference in source.items():
                value = dict(reference)
                rows.append(
                    {
                        "key": key,
                        "title": _first(value.get("title")),
                        "doi": _first(value.get("DOI")),
                        "volume": str(_first(value.get("volume"))),
                    }
                )
            return OperationOutcome(rows, len(rows))

        return _prepared(
            "inspect",
            operation,
            _projection_contains(
                workload.records,
                required_fields=("key", "title", "doi", "volume"),
            ),
        )


class BibtexparserAdapter(PackageAdapter):
    name = "bibtexparser-1.x"
    distribution = "bibtexparser"

    def prepare_bibtex_parse(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        def operation() -> OperationOutcome:
            with workload.bibtex_path.open(encoding="utf-8") as handle:
                database = bibtexparser.load(handle)
            return OperationOutcome(database, len(database.entries))

        return _prepared("parse", operation, _count_is(len(workload.records)), setup_included=True)

    def prepare_bibtex_recovery_parse(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import bibtexparser

        def operation() -> OperationOutcome:
            try:
                with workload.dirty_bibtex_path.open(encoding="utf-8") as handle:
                    database = bibtexparser.load(handle)
            except Exception as exc:
                return OperationOutcome("", 0, _error_detail(exc))
            return OperationOutcome(database, len(database.entries))

        return _prepared(
            "parse-recovery",
            operation,
            _recovery_parse_result(len(workload.records)),
            source_format="dirty_bibtex",
            setup_included=True,
        )

    def prepare_raw_bibtex_parse(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        def operation() -> OperationOutcome:
            database = bibtexparser.loads(workload.raw_bibtex)
            return OperationOutcome(database, len(database.entries))

        return _prepared(
            "raw-parse",
            operation,
            _count_is(len(workload.records)),
            source_format="raw_bibtex",
            setup_included=True,
        )

    def prepare_raw_bibtex_write(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        database = bibtexparser.loads(workload.raw_bibtex)
        database.entries[0]["title"] = "Edited Benchmark Title"

        def operation() -> OperationOutcome:
            text = bibtexparser.dumps(database)
            path = directory / f"bibtexparser-raw-write-{perf_counter_ns()}.bib"
            path.write_text(text, encoding="utf-8")
            return OperationOutcome(path, len(database.entries), path.name)

        return _prepared(
            "raw-write",
            operation,
            _raw_roundtrip_check(workload.keys),
            source_format="raw_bibtex",
        )

    def prepare_raw_bibtex_roundtrip(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import bibtexparser

        def operation() -> OperationOutcome:
            database = bibtexparser.loads(workload.raw_bibtex)
            database.entries[0]["title"] = "Edited Benchmark Title"
            text = bibtexparser.dumps(database)
            path = directory / f"bibtexparser-raw-{perf_counter_ns()}.bib"
            path.write_text(text, encoding="utf-8")
            return OperationOutcome(path, len(database.entries), path.name)

        return _prepared(
            "raw-write",
            operation,
            _raw_roundtrip_check(workload.keys),
            source_format="raw_bibtex",
            setup_included=True,
        )

    def prepare_citation_render(self, workload: Workload, directory: Path) -> PreparedOperation:
        raise UnsupportedOperation("bibtexparser does not render CSL citations")

    def prepare_bibliography_render(self, workload: Workload, directory: Path) -> PreparedOperation:
        raise UnsupportedOperation("bibtexparser does not render CSL bibliographies")

    def prepare_repeated_render(self, workload: Workload, directory: Path) -> PreparedOperation:
        raise UnsupportedOperation("bibtexparser does not render CSL citations")

    def prepare_one_off_cite(self, workload: Workload, directory: Path) -> PreparedOperation:
        raise UnsupportedOperation("bibtexparser does not render CSL citations")

    def prepare_one_off_bibliography(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        raise UnsupportedOperation("bibtexparser does not render CSL bibliographies")

    def prepare_missing_reference(self, workload: Workload, directory: Path) -> PreparedOperation:
        raise UnsupportedOperation("bibtexparser has no citation reference resolution step")

    def prepare_bulk_materialization(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import bibtexparser

        database = bibtexparser.loads(workload.bibtex)

        def operation() -> OperationOutcome:
            rows = [{"key": entry["ID"], "title": entry.get("title")} for entry in database.entries]
            return OperationOutcome(rows, len(rows))

        return _prepared(
            "materialize",
            operation,
            _projection_contains(workload.records, required_fields=("key", "title")),
        )

    def prepare_library_keys(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        database = bibtexparser.loads(workload.bibtex)

        def operation() -> OperationOutcome:
            keys = [entry["ID"] for entry in database.entries]
            return OperationOutcome(keys, len(keys))

        return _prepared("inspect", operation, _keys_are(workload.keys))

    def prepare_entry_lookup(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        database = bibtexparser.loads(workload.bibtex)
        entries = {entry["ID"]: entry for entry in database.entries}
        keys = _lookup_keys(workload)

        def operation() -> OperationOutcome:
            rows = [entries[key] for key in keys]
            return OperationOutcome(rows, len(rows))

        return _prepared("inspect", operation, _entries_match(workload.records[: len(keys)]))

    def prepare_field_projection(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        database = bibtexparser.loads(workload.bibtex)

        def operation() -> OperationOutcome:
            rows = [
                {
                    "key": entry["ID"],
                    "title": entry.get("title"),
                    "doi": entry.get("doi"),
                    "volume": entry.get("volume"),
                }
                for entry in database.entries
            ]
            return OperationOutcome(rows, len(rows))

        return _prepared(
            "inspect",
            operation,
            _projection_contains(
                workload.records,
                required_fields=("key", "title", "doi", "volume"),
            ),
        )


def adapters() -> list[PackageAdapter]:
    return [RefkitAdapter(), CiteprocPyAdapter(), BibtexparserAdapter()]


def _prepared(
    phase: str,
    operation: Callable[[], OperationOutcome],
    check: Callable[[OperationOutcome], None],
    *,
    source_format: str = "bibtex",
    setup_included: bool = False,
    citation_count: int = 0,
) -> PreparedOperation:
    return PreparedOperation(
        phase=phase,
        operation=operation,
        check=check,
        metadata={
            "source_format": source_format,
            "setup_included": setup_included,
            "citation_count": citation_count,
        },
    )


def _count_is(expected: int) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        if outcome.count != expected:
            raise AssertionError(f"expected count {expected}, got {outcome.count}")

    return check


def _count_at_least(minimum: int) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        if outcome.count < minimum:
            raise AssertionError(f"expected count at least {minimum}, got {outcome.count}")

    return check


def _recovery_parse_result(expected: int) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        if outcome.count == expected:
            return
        if outcome.count == 0 and outcome.detail.startswith("error="):
            return
        raise AssertionError(f"expected {expected} recovered entries or recorded parse error")

    return check


def _keys_are(expected: list[str]) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        if outcome.value != expected:
            raise AssertionError("expected keys to match fixture order")
        if outcome.count != len(expected):
            raise AssertionError(f"expected count {len(expected)}, got {outcome.count}")

    return check


def _all_checks(*checks: Callable[[OperationOutcome], None]) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        for item in checks:
            item(outcome)

    return check


def _projection_contains(
    records: tuple[Any, ...],
    *,
    required_fields: tuple[str, ...] = (),
) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        rows = list(cast(Iterable[Mapping[str, Any]], outcome.value))
        if len(rows) != len(records):
            raise AssertionError(f"expected {len(records)} projected rows, got {len(rows)}")
        by_key = {str(row["key"]): row for row in rows}
        for record in records:
            row = by_key.get(record.key)
            if row is None:
                raise AssertionError(f"expected projected rows to contain {record.key!r}")
            for required_field in required_fields:
                if required_field not in row:
                    raise AssertionError(
                        f"expected projected row for {record.key!r} to include {required_field!r}"
                    )
            if row.get("title") != record.title:
                raise AssertionError(f"expected title for {record.key!r}")
            doi = row.get("doi")
            if ("doi" in required_fields and doi is None) or (
                doi is not None and str(doi) != record.doi
            ):
                raise AssertionError(f"expected DOI for {record.key!r}")
            volume = row.get("volume")
            if ("volume" in required_fields and volume is None) or (
                volume is not None and str(volume) != str(record.volume)
            ):
                raise AssertionError(f"expected volume for {record.key!r}")

    return check


def _entries_match(records: tuple[Any, ...]) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        rows = list(cast(Iterable[Any], outcome.value))
        if len(rows) != len(records):
            raise AssertionError(f"expected {len(records)} entries, got {len(rows)}")
        for row, record in zip(rows, records, strict=True):
            key = getattr(row, "key", None)
            title = getattr(row, "title", None)
            if key is None and isinstance(row, Mapping):
                key = row.get("ID") or row.get("id") or row.get("key")
            if title is None and isinstance(row, Mapping):
                title = row.get("title")
            if key != record.key:
                raise AssertionError(f"expected entry key {record.key!r}")
            if _first(title) != record.title:
                raise AssertionError(f"expected title for {record.key!r}")

    return check


def _text_contains(needle: str) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        if needle not in str(outcome.value):
            raise AssertionError(f"expected output to contain {needle!r}")

    return check


def _text_contains_all(needles: list[str]) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        text = str(outcome.value)
        missing = [needle for needle in needles if needle not in text]
        if missing:
            raise AssertionError(f"expected output to contain {missing[0]!r}")

    return check


def _citation_output_matches(records: tuple[Any, ...]) -> Callable[[OperationOutcome], None]:
    expected = [f"({record.family}, {record.year})" for record in records]

    def check(outcome: OperationOutcome) -> None:
        lines = _non_empty_lines(str(outcome.value))
        if lines != expected:
            raise AssertionError("expected rendered citations to match fixture order and APA shape")

    return check


def _bibliography_output_matches(records: tuple[Any, ...]) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        rows = _non_empty_lines(str(outcome.value))
        if len(rows) != len(records):
            raise AssertionError(f"expected {len(records)} bibliography rows, got {len(rows)}")
        for row, record in zip(rows, records, strict=True):
            normalized = row.replace("\u2013", "-")
            expected_values = [
                record.family,
                str(record.year),
                record.title,
                "Journal of Citation Benchmarks",
                str(record.volume),
                f"{record.page_start}-{record.page_end}",
                record.doi,
            ]
            missing = [value for value in expected_values if value not in normalized]
            if missing:
                raise AssertionError(
                    f"expected bibliography row for {record.key!r} to contain {missing[0]!r}"
                )

    return check


def _non_empty_lines(value: str) -> list[str]:
    return [line.strip() for line in value.splitlines() if line.strip()]


def _detail_contains(needle: str) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        if needle not in outcome.detail:
            raise AssertionError(f"expected detail to contain {needle!r}")

    return check


def _error_detail(exc: Exception) -> str:
    return f"error={type(exc).__name__}: {exc!r}"


def _raw_roundtrip_check(keys: list[str]) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        path = Path(str(outcome.value))
        text = path.read_text(encoding="utf-8")
        expected = [
            "Edited Benchmark Title",
            "benchmark fixture with raw BibTeX blocks",
            "benchjournal",
            "Reference benchmark fixture",
            *keys,
        ]
        missing = [value for value in expected if value not in text]
        if missing:
            raise AssertionError(f"expected written file to contain {missing[0]!r}")
        if text.lower().count("@article{") != len(keys):
            raise AssertionError(f"expected {len(keys)} written entries")

    return check


def _lookup_keys(workload: Workload) -> list[str]:
    return workload.keys[: min(16, len(workload.keys))]


def _first(value: object) -> object:
    if isinstance(value, list) and type(value) is list:
        return value[0] if value else None
    if isinstance(value, list):
        return str(value)
    return value


def _citeproc_source_and_style(workload: Workload) -> tuple[Any, Any]:
    from citeproc import CitationStylesStyle
    from citeproc.source.json import CiteProcJSON

    source = CiteProcJSON(workload.csl_json)
    style = CitationStylesStyle("apa", validate=False)
    return source, style


def _citeproc_processor(source: Any, style: Any) -> Any:
    from citeproc import CitationStylesBibliography, formatter

    return CitationStylesBibliography(style, source, formatter.plain)


def _citeproc_citation(keys: list[str]) -> Any:
    from citeproc import Citation, CitationItem

    return Citation([CitationItem(key) for key in keys])
