from __future__ import annotations

from collections.abc import Iterable, Sequence
from pathlib import Path
from typing import Any, Literal

from refkit_bench._adapters.common import (
    OperationOutcome,
    PackageAdapter,
    PreparedOperation,
    _all_checks,
    _bibliography_output_matches,
    _citation_output_matches,
    _count_is,
    _entries_match,
    _keys_are,
    _lookup_keys,
    _prepared,
    _projection_contains,
)
from refkit_bench.fixtures import Workload

ExecutionMode = Literal["eager", "lazy"]


class PolarsRefkitAdapter(PackageAdapter):
    name = "polars-refkit"
    distribution = "polars-refkit"

    def prepare_parse_bibtex(self, workload: Workload, directory: Path) -> PreparedOperation:
        return self._prepare_parse_bibtex(workload, lazy=False)

    def prepare_render_citation_expression_eager(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        return self._prepare_render_citation_expression(workload, lazy=False)

    def prepare_render_bibliography_expression_eager(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        return self._prepare_render_bibliography_expression(workload, lazy=False)

    def prepare_render_citation_each_eager(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        return self._prepare_render_repeated_citations(workload, lazy=False)

    def prepare_materialize_entry_rows_eager(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        return self._prepare_materialize_entry_rows(workload, lazy=False)

    def prepare_list_keys_eager(self, workload: Workload, directory: Path) -> PreparedOperation:
        return self._prepare_list_keys(workload, lazy=False)

    def prepare_lookup_entries_eager(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        return self._prepare_lookup_entries(workload, lazy=False)

    def prepare_project_fields_eager(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        return self._prepare_project_fields(workload, lazy=False)

    def prepare_parse_bibtex_lazy(self, workload: Workload, directory: Path) -> PreparedOperation:
        return self._prepare_parse_bibtex(workload, lazy=True)

    def prepare_render_citation_expression_lazy(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        return self._prepare_render_citation_expression(workload, lazy=True)

    def prepare_render_bibliography_expression_lazy(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        return self._prepare_render_bibliography_expression(workload, lazy=True)

    def prepare_render_citation_each_lazy(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        return self._prepare_render_repeated_citations(workload, lazy=True)

    def prepare_materialize_entry_rows_lazy(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        return self._prepare_materialize_entry_rows(workload, lazy=True)

    def prepare_list_keys_lazy(self, workload: Workload, directory: Path) -> PreparedOperation:
        return self._prepare_list_keys(workload, lazy=True)

    def prepare_lookup_entries_lazy(self, workload: Workload, directory: Path) -> PreparedOperation:
        return self._prepare_lookup_entries(workload, lazy=True)

    def prepare_project_fields_lazy(self, workload: Workload, directory: Path) -> PreparedOperation:
        return self._prepare_project_fields(workload, lazy=True)

    def _prepare_parse_bibtex(self, workload: Workload, *, lazy: bool) -> PreparedOperation:
        import polars_refkit as prk

        def operation() -> OperationOutcome:
            source = workload.bibtex_path.read_text(encoding="utf-8")
            frame = _frame({"bibtex": [source]}, lazy=lazy)
            count = _select(frame, lazy=lazy, count=prk.entry_count("bibtex")).item()
            return OperationOutcome(count, int(count or 0))

        return _prepared(
            operation,
            _count_is(len(workload.records)),
            setup_included=True,
            execution_mode=_execution_mode(lazy),
        )

    def _prepare_render_citation_expression(
        self, workload: Workload, *, lazy: bool
    ) -> PreparedOperation:
        import polars_refkit as prk

        frame = _frame({"bibtex": [workload.bibtex], "key": [workload.keys[0]]}, lazy=lazy)

        def operation() -> OperationOutcome:
            result = _select(frame, lazy=lazy, citation=prk.cite("bibtex", "key"))
            citation = result["citation"].item()
            return OperationOutcome(citation, 1)

        return _prepared(
            operation,
            _all_checks(_count_is(1), _citation_output_matches(workload.records[:1])),
            setup_included=True,
            citation_count=1,
            execution_mode=_execution_mode(lazy),
        )

    def _prepare_render_bibliography_expression(
        self, workload: Workload, *, lazy: bool
    ) -> PreparedOperation:
        import polars_refkit as prk

        frame = _frame({"bibtex": [workload.bibtex]}, lazy=lazy)

        def operation() -> OperationOutcome:
            result = _select(
                frame,
                lazy=lazy,
                bibliography=prk.full_bibliography_text("bibtex"),
            )
            bibliography = result["bibliography"].item()
            return OperationOutcome(bibliography, len(workload.records))

        return _prepared(
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibliography_output_matches(workload.records),
            ),
            setup_included=True,
            citation_count=0,
            execution_mode=_execution_mode(lazy),
        )

    def _prepare_render_repeated_citations(
        self, workload: Workload, *, lazy: bool
    ) -> PreparedOperation:
        import polars_refkit as prk

        frame = _frame({"bibtex": [workload.bibtex], "keys": [workload.keys]}, lazy=lazy)

        def operation() -> OperationOutcome:
            result = _select(frame, lazy=lazy, citations=prk.cite_each("bibtex", "keys"))
            texts = result["citations"].to_list()[0]
            return OperationOutcome("\n".join(texts), len(texts))

        return _prepared(
            operation,
            _all_checks(
                _count_is(len(workload.keys)),
                _citation_output_matches(workload.records[: len(workload.keys)]),
            ),
            source_format="bibtex",
            setup_included=True,
            citation_count=len(workload.keys),
            execution_mode=_execution_mode(lazy),
        )

    def _prepare_materialize_entry_rows(
        self, workload: Workload, *, lazy: bool
    ) -> PreparedOperation:
        frame = _frame({"bibtex": [workload.bibtex]}, lazy=lazy)

        def operation() -> OperationOutcome:
            result = _entries_frame(
                frame,
                "bibtex",
                fields=("key", "title"),
                lazy=lazy,
            )
            rows = result.to_dicts()
            return OperationOutcome(rows, len(rows))

        return _prepared(
            operation,
            _projection_contains(workload.records, required_fields=("key", "title")),
            source_format="bibtex",
            setup_included=True,
            execution_mode=_execution_mode(lazy),
        )

    def _prepare_list_keys(self, workload: Workload, *, lazy: bool) -> PreparedOperation:
        import polars_refkit as prk

        frame = _frame({"bibtex": [workload.bibtex]}, lazy=lazy)

        def operation() -> OperationOutcome:
            result = _select(frame, lazy=lazy, keys=prk.keys("bibtex"))
            keys = result["keys"].to_list()[0]
            return OperationOutcome(keys, len(keys))

        return _prepared(
            operation,
            _keys_are(workload.keys),
            source_format="bibtex",
            setup_included=True,
            execution_mode=_execution_mode(lazy),
        )

    def _prepare_lookup_entries(self, workload: Workload, *, lazy: bool) -> PreparedOperation:
        keys = _lookup_keys(workload)
        frame = _frame({"bibtex": [workload.bibtex]}, lazy=lazy)

        def operation() -> OperationOutcome:
            result = _entries_frame(
                frame,
                "bibtex",
                fields=("key", "title"),
                lazy=lazy,
                filter_keys=keys,
            )
            rows = result.to_dicts()
            return OperationOutcome(rows, len(rows))

        return _prepared(
            operation,
            _entries_match(workload.records[: len(keys)]),
            source_format="bibtex",
            setup_included=True,
            execution_mode=_execution_mode(lazy),
        )

    def _prepare_project_fields(self, workload: Workload, *, lazy: bool) -> PreparedOperation:
        frame = _frame({"bibtex": [workload.bibtex]}, lazy=lazy)

        def operation() -> OperationOutcome:
            result = _entries_frame(
                frame,
                "bibtex",
                fields=("key", "title", "doi", "volume"),
                lazy=lazy,
            )
            rows = result.to_dicts()
            return OperationOutcome(rows, len(rows))

        return _prepared(
            operation,
            _projection_contains(
                workload.records,
                required_fields=("key", "title", "doi"),
            ),
            source_format="bibtex",
            setup_included=True,
            execution_mode=_execution_mode(lazy),
        )


def _execution_mode(lazy: bool) -> ExecutionMode:
    return "lazy" if lazy else "eager"


def _frame(data: dict[str, Sequence[Any]], *, lazy: bool) -> Any:
    import polars as pl

    frame = pl.DataFrame(data)
    if lazy:
        return frame.lazy()
    return frame


def _select(frame: Any, *, lazy: bool, **expressions: Any) -> Any:
    result = frame.select(**expressions)
    if lazy:
        return result.collect()
    return result


def _entries_frame(
    frame: Any,
    column: str,
    *,
    fields: Iterable[str],
    lazy: bool,
    filter_keys: Sequence[str] = (),
) -> Any:
    import polars as pl

    result = _polars_entries_frame(frame, column, fields=fields)
    if filter_keys:
        result = result.filter(pl.col("key").is_in(filter_keys))
    if lazy:
        return result.collect()
    return result


def _polars_entries_frame(frame: Any, column: str, *, fields: Iterable[str]) -> Any:
    import polars_refkit as prk

    return (
        frame.select(entries=prk.entries(column, fields=fields))
        .explode("entries")
        .unnest("entries")
    )
