from __future__ import annotations

from pathlib import Path
from typing import Any

from benchmark._adapters.common import (
    OperationOutcome,
    PackageAdapter,
    PreparedOperation,
    UnsupportedOperation,
    _all_checks,
    _bibliography_output_matches,
    _citation_output_matches,
    _count_is,
    _detail_contains,
    _entries_match,
    _first,
    _keys_are,
    _lookup_keys,
    _prepared,
    _projection_contains,
)
from benchmark.fixtures import Workload


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
        raise UnsupportedOperation(
            "citeproc-py aborts dirty BibTeX instead of recovering valid entries"
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
