from __future__ import annotations

from pathlib import Path
from typing import Any

from refkit_bench._adapters.common import (
    MissingBenchmarkOperation,
    OperationOutcome,
    PackageAdapter,
    PreparedOperation,
    _all_checks,
    _bibliography_output_matches,
    _citation_output_matches,
    _count_is,
    _detail_contains,
    _prepared,
)
from refkit_bench.fixtures import Workload


class CiteprocPyAdapter(PackageAdapter):
    name = "citeproc-py"
    distribution = "citeproc-py"

    def prepare_load_bundled_style(self, workload: Workload, directory: Path) -> PreparedOperation:
        from citeproc import CitationStylesStyle

        def operation() -> OperationOutcome:
            style = CitationStylesStyle("apa", validate=False)
            return OperationOutcome(style, 1)

        return _prepared(operation, _count_is(1), source_format="none")

    def prepare_create_processor(self, workload: Workload, directory: Path) -> PreparedOperation:
        source, style = _citeproc_source_and_style(workload)

        def operation() -> OperationOutcome:
            bibliography = _citeproc_processor(source, style)
            return OperationOutcome(bibliography, 1)

        return _prepared(operation, _count_is(1), source_format="csl_json")

    def prepare_render_one_prepared_citation(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        source, style = _citeproc_source_and_style(workload)
        citation = _citeproc_citation(workload.keys[:1])

        def operation() -> OperationOutcome:
            bibliography = _citeproc_processor(source, style)
            bibliography.register(citation)
            rendered = bibliography.cite(citation, lambda item: None)
            return OperationOutcome(str(rendered), 1)

        return _prepared(
            operation,
            _all_checks(_count_is(1), _citation_output_matches(workload.records[:1])),
            source_format="csl_json",
            citation_count=1,
        )

    def prepare_render_prepared_bibliography(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        source, style = _citeproc_source_and_style(workload)
        citation = _citeproc_citation(workload.keys)

        def operation() -> OperationOutcome:
            bibliography = _citeproc_processor(source, style)
            bibliography.register(citation)
            bibliography.sort()
            rows = [str(item) for item in bibliography.bibliography()]
            return OperationOutcome("\n".join(rows), len(rows))

        return _prepared(
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibliography_output_matches(workload.records),
            ),
            source_format="csl_json",
            citation_count=len(workload.records),
        )

    def prepare_render_cited_bibliography(
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
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibliography_output_matches(workload.records),
            ),
            source_format="csl_json",
            citation_count=len(workload.records),
        )

    def prepare_render_repeated_citations(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
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
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _citation_output_matches(workload.records),
            ),
            source_format="csl_json",
            citation_count=len(workload.records),
        )

    def prepare_render_path_citation(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
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
            operation,
            _all_checks(_count_is(1), _citation_output_matches(workload.records[:1])),
            setup_included=True,
            citation_count=1,
        )

    def prepare_render_path_bibliography(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        if workload.family == "real_bibliography_subset":
            raise MissingBenchmarkOperation(
                "citeproc-py BibTeX source expands this real bibliography subset into "
                "non-entry bibliography rows"
            )

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
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibliography_output_matches(workload.records),
            ),
            setup_included=True,
            citation_count=len(workload.records),
        )

    def prepare_resolve_missing_reference(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        source, style = _citeproc_source_and_style(workload)
        citation = _citeproc_citation(["missing-reference"])

        def operation() -> OperationOutcome:
            bibliography = _citeproc_processor(source, style)
            bibliography.register(citation)
            missing: list[str] = []
            rendered = bibliography.cite(citation, lambda item: missing.append(item.key))
            return OperationOutcome(str(rendered), len(missing), ",".join(missing))

        return _prepared(
            operation,
            _all_checks(_count_is(1), _detail_contains("missing-reference")),
            source_format="csl_json",
            citation_count=1,
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
