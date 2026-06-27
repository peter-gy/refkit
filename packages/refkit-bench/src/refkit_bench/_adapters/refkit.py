from __future__ import annotations

from pathlib import Path
from time import perf_counter_ns
from typing import Any

from refkit_bench._adapters.common import (
    OperationOutcome,
    PackageAdapter,
    PreparedOperation,
    _all_checks,
    _bibliography_output_matches,
    _citation_output_matches,
    _count_at_least,
    _count_is,
    _duplicate_signals_cover,
    _entries_match,
    _keys_are,
    _lookup_keys,
    _prepared,
    _projection_contains,
    _raw_blocks_cover,
    _raw_roundtrip_check,
    _text_contains,
)
from refkit_bench.fixtures import Workload


class RefkitAdapter(PackageAdapter):
    name = "refkit"
    distribution = "refkit"

    def prepare_parse_bibtex(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        def operation() -> OperationOutcome:
            library = rk.Library.read(workload.bibtex_path)
            return OperationOutcome(library, len(library))

        return _prepared(operation, _count_is(len(workload.records)), setup_included=True)

    def prepare_parse_bibtex_text(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        def operation() -> OperationOutcome:
            library = rk.Library.parse_bibtex(workload.bibtex)
            return OperationOutcome(library, len(library))

        return _prepared(operation, _count_is(len(workload.records)), setup_included=True)

    def prepare_recover_dirty_bibtex(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        def operation() -> OperationOutcome:
            library = rk.Library.read(workload.dirty_bibtex_path, recovery="report")
            return OperationOutcome(
                library,
                len(library),
                f"diagnostics={len(library.diagnostics)}",
                metadata={"diagnostic_count": len(library.diagnostics)},
            )

        return _prepared(
            operation,
            _count_is(len(workload.records)),
            source_format="dirty_bibtex",
            setup_included=True,
        )

    def prepare_extract_diagnostics(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        expected = 0 if workload.dirty_bibtex == workload.bibtex else 4

        def operation() -> OperationOutcome:
            library = rk.Library.read(workload.dirty_bibtex_path, recovery="report")
            diagnostics = [{"message": message} for message in library.diagnostics]
            return OperationOutcome(
                diagnostics,
                len(diagnostics),
                metadata={"diagnostic_count": len(diagnostics)},
            )

        return _prepared(
            operation,
            _count_is(expected),
            source_format="dirty_bibtex",
            setup_included=True,
        )

    def prepare_parse_raw_bibtex(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        def operation() -> OperationOutcome:
            document = rk.BibDocument.parse(workload.raw_bibtex)
            return OperationOutcome(document, len(document.entries))

        return _prepared(
            operation,
            _count_is(len(workload.records)),
            source_format="raw_bibtex",
            setup_included=True,
        )

    def prepare_materialize_raw_blocks(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        document = rk.BibDocument.parse(workload.raw_bibtex)

        def operation() -> OperationOutcome:
            rows = [
                {
                    "kind": block["kind"],
                    "key": block.get("key", ""),
                    "raw_bytes": len(str(block.get("raw", "")).encode("utf-8")),
                }
                for block in document.blocks
                if block["kind"] != "whitespace"
            ]
            return OperationOutcome(rows, len(rows))

        return _prepared(
            operation,
            _raw_blocks_cover(workload),
            source_format="raw_bibtex",
        )

    def prepare_handle_duplicates(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        document = rk.BibDocument.parse(workload.duplicate_bibtex)

        def operation() -> OperationOutcome:
            duplicate_entries = document.entries.get_all(workload.duplicate_entry_key)
            duplicate_field_entry = document.entries[workload.duplicate_field_key]
            duplicate_fields = duplicate_field_entry.fields.get_all(workload.duplicate_field_name)
            rows = [
                {
                    "kind": "duplicate_entry",
                    "key": workload.duplicate_entry_key,
                    "field": "",
                    "count": len(duplicate_entries),
                },
                {
                    "kind": "duplicate_field",
                    "key": workload.duplicate_field_key,
                    "field": workload.duplicate_field_name,
                    "count": len(duplicate_fields),
                },
            ]
            return OperationOutcome(rows, len(rows))

        return _prepared(
            operation,
            _duplicate_signals_cover(workload),
            source_format="duplicate_bibtex",
        )

    def prepare_write_edited_raw_bibtex(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        document = rk.BibDocument.parse(workload.raw_bibtex)
        field = document.entries[workload.keys[0]].fields["title"]

        def operation() -> OperationOutcome:
            field.value = "Edited Benchmark Title"
            path = directory / f"refkit-raw-write-{perf_counter_ns()}.bib"
            document.write(path)
            return OperationOutcome(path, len(document.entries), path.name)

        return _prepared(
            operation,
            _raw_roundtrip_check(workload.keys, workload.raw_preservation_terms),
            source_format="raw_bibtex",
        )

    def prepare_roundtrip_raw_bibtex_edit(
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
            operation,
            _raw_roundtrip_check(workload.keys, workload.raw_preservation_terms),
            source_format="raw_bibtex",
            setup_included=True,
        )

    def prepare_load_bundled_style(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        def operation() -> OperationOutcome:
            style = rk.Style.load("apa")
            return OperationOutcome(style, 1)

        return _prepared(operation, _count_is(1), source_format="none")

    def prepare_create_processor(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse_bibtex(workload.bibtex)
        style = rk.Style.load("apa")

        def operation() -> OperationOutcome:
            document = rk.Document(library, style, locale="en-US")
            return OperationOutcome(document, 1)

        return _prepared(operation, _count_is(1))

    def prepare_render_one_prepared_citation(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse_bibtex(workload.bibtex)
        style = rk.Style.load("apa")
        key = workload.keys[0]

        def operation() -> OperationOutcome:
            document = rk.Document(library, style, locale="en-US")
            rendered = document.render([rk.Citation("citation", key)])["citation"]
            return OperationOutcome(rendered.text, 1)

        return _prepared(
            operation,
            _all_checks(_count_is(1), _citation_output_matches(workload.records[:1])),
            citation_count=1,
        )

    def prepare_render_prepared_bibliography(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse_bibtex(workload.bibtex)
        style = rk.Style.load("apa")

        def operation() -> OperationOutcome:
            document = rk.Document(library, style, locale="en-US")
            rendered = document.full_bibliography()
            return OperationOutcome(rendered.text, len(workload.records))

        return _prepared(
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibliography_output_matches(workload.records),
            ),
            citation_count=0,
        )

    def prepare_render_cited_bibliography(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse_bibtex(workload.bibtex)
        style = rk.Style.load("apa")
        keys = workload.keys

        def operation() -> OperationOutcome:
            document = rk.Document(library, style, locale="en-US")
            citations = [rk.Citation(f"citation_{index}", key) for index, key in enumerate(keys)]
            rendered = document.cited_bibliography(citations)
            return OperationOutcome(rendered.text, len(keys))

        return _prepared(
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibliography_output_matches(workload.records),
            ),
            citation_count=len(workload.records),
        )

    def prepare_render_repeated_citations(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse_bibtex(workload.bibtex)
        style = rk.Style.load("apa")
        keys = workload.keys

        def operation() -> OperationOutcome:
            document = rk.Document(library, style, locale="en-US")
            citations = [rk.Citation(f"citation_{index}", key) for index, key in enumerate(keys)]
            rendered = document.render(citations)
            texts = [rendered[f"citation_{index}"].text for index in range(len(keys))]
            return OperationOutcome("\n".join(texts), len(texts))

        return _prepared(
            operation,
            _all_checks(
                _count_is(len(keys)),
                _citation_output_matches(workload.records[: len(keys)]),
            ),
            citation_count=len(keys),
        )

    def prepare_access_rendered_text(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        rendered = self._prepared_rendered_citation(workload)

        def operation() -> OperationOutcome:
            text = rendered.text
            return OperationOutcome(text, 1)

        return _prepared(
            operation,
            _all_checks(_count_is(1), _citation_output_matches(workload.records[:1])),
        )

    def prepare_access_rendered_html(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        rendered = self._prepared_rendered_citation(workload)

        def operation() -> OperationOutcome:
            html = rendered.html
            return OperationOutcome(html, 1)

        return _prepared(operation, _text_contains(workload.records[0].family))

    def prepare_access_rendered_tree(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        rendered = self._prepared_rendered_citation(workload)

        def operation() -> OperationOutcome:
            tree = rendered.tree
            return OperationOutcome(tree, len(tree))

        return _prepared(operation, _count_at_least(1))

    def _prepared_rendered_citation(self, workload: Workload) -> Any:
        import refkit as rk

        library = rk.Library.parse_bibtex(workload.bibtex)
        style = rk.Style.load("apa")
        document = rk.Document(library, style, locale="en-US")
        return document.render([rk.Citation("citation", workload.keys[0])])["citation"]

    def prepare_render_path_citation(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        key = workload.keys[0]

        def operation() -> OperationOutcome:
            rendered = rk.cite(workload.bibtex_path, key, style="apa")
            return OperationOutcome(rendered.text, 1)

        return _prepared(
            operation,
            _all_checks(_count_is(1), _citation_output_matches(workload.records[:1])),
            setup_included=True,
            citation_count=1,
        )

    def prepare_render_path_bibliography(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        def operation() -> OperationOutcome:
            rendered = rk.full_bibliography(workload.bibtex_path, style="apa")
            return OperationOutcome(rendered.text, len(workload.records))

        return _prepared(
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibliography_output_matches(workload.records),
            ),
            setup_included=True,
            citation_count=0,
        )

    def prepare_resolve_missing_reference(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse_bibtex(workload.bibtex)
        style = rk.Style.load("apa")

        def operation() -> OperationOutcome:
            document = rk.Document(library, style, locale="en-US")
            try:
                document.render([rk.Citation("missing", "missing-reference")])
            except rk.MissingReferenceError as exc:
                return OperationOutcome(str(exc), 1, "raised")
            raise AssertionError("missing reference did not raise")  # pragma: no cover

        return _prepared(
            operation,
            _all_checks(_count_is(1), _text_contains("missing-reference")),
            citation_count=1,
        )

    def prepare_materialize_entry_rows(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse_bibtex(workload.bibtex)

        def operation() -> OperationOutcome:
            rows = library.project(["key", "title"])
            return OperationOutcome(rows, len(rows))

        return _prepared(
            operation,
            _projection_contains(workload.records, required_fields=("key", "title")),
        )

    def prepare_list_keys(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse_bibtex(workload.bibtex)

        def operation() -> OperationOutcome:
            keys = library.keys()
            return OperationOutcome(keys, len(keys))

        return _prepared(operation, _keys_are(workload.keys))

    def prepare_lookup_entries(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse_bibtex(workload.bibtex)
        keys = _lookup_keys(workload)

        def operation() -> OperationOutcome:
            rows = library.get_many(keys)
            return OperationOutcome(rows, len(rows))

        return _prepared(operation, _entries_match(workload.records[: len(keys)]))

    def prepare_project_fields(self, workload: Workload, directory: Path) -> PreparedOperation:
        import refkit as rk

        library = rk.Library.parse_bibtex(workload.bibtex)

        def operation() -> OperationOutcome:
            rows = library.project(["key", "title", "doi", "volume"])
            return OperationOutcome(rows, len(rows))

        return _prepared(
            operation,
            _projection_contains(
                workload.records,
                required_fields=("key", "title", "doi"),
            ),
        )
