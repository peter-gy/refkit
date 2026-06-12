from __future__ import annotations

from pathlib import Path
from time import perf_counter_ns
from typing import Any

from benchmark._adapters.common import (
    OperationOutcome,
    PackageAdapter,
    PreparedOperation,
    _all_checks,
    _bibliography_output_matches,
    _citation_output_matches,
    _count_at_least,
    _count_is,
    _entries_match,
    _keys_are,
    _lookup_keys,
    _prepared,
    _projection_contains,
    _raw_roundtrip_check,
    _text_contains,
)
from benchmark.fixtures import Workload


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
