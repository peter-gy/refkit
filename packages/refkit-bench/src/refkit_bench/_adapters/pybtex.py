from __future__ import annotations

from pathlib import Path
from typing import Any

from refkit_bench._adapters.common import (
    OperationOutcome,
    PackageAdapter,
    PreparedOperation,
    _entries_match,
    _keys_are,
    _lookup_keys,
    _parsed_records_match,
    _prepared,
    _projection_contains,
)
from refkit_bench.fixtures import Workload


class PybtexAdapter(PackageAdapter):
    name = "pybtex"
    distribution = "pybtex"

    def prepare_parse_bibtex(self, workload: Workload, directory: Path) -> PreparedOperation:
        from pybtex.database.input import bibtex

        def operation() -> OperationOutcome:
            database = bibtex.Parser().parse_file(str(workload.bibtex_path))
            return OperationOutcome(database, len(database.entries))

        return _prepared(
            operation, _parsed_records_match(workload.records, _parsed_rows), setup_included=True
        )

    def prepare_parse_bibtex_text(self, workload: Workload, directory: Path) -> PreparedOperation:
        from pybtex.database.input import bibtex

        def operation() -> OperationOutcome:
            database = bibtex.Parser().parse_string(workload.bibtex)
            return OperationOutcome(database, len(database.entries))

        return _prepared(
            operation, _parsed_records_match(workload.records, _parsed_rows), setup_included=True
        )

    def prepare_materialize_entry_rows(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        database = _parse_bibtex_string(workload.bibtex)

        def operation() -> OperationOutcome:
            rows = [
                {"key": key, "title": entry.fields.get("title")}
                for key, entry in database.entries.items()
            ]
            return OperationOutcome(rows, len(rows))

        return _prepared(
            operation,
            _projection_contains(workload.records, required_fields=("key", "title")),
        )

    def prepare_list_keys(self, workload: Workload, directory: Path) -> PreparedOperation:
        database = _parse_bibtex_string(workload.bibtex)

        def operation() -> OperationOutcome:
            keys = list(database.entries)
            return OperationOutcome(keys, len(keys))

        return _prepared(operation, _keys_are(workload.keys))

    def prepare_lookup_entries(self, workload: Workload, directory: Path) -> PreparedOperation:
        database = _parse_bibtex_string(workload.bibtex)
        keys = _lookup_keys(workload)

        def operation() -> OperationOutcome:
            rows = [
                {
                    "key": key,
                    "title": database.entries[key].fields.get("title"),
                }
                for key in keys
            ]
            return OperationOutcome(rows, len(rows))

        return _prepared(operation, _entries_match(workload.records[: len(keys)]))

    def prepare_project_fields(self, workload: Workload, directory: Path) -> PreparedOperation:
        database = _parse_bibtex_string(workload.bibtex)

        def operation() -> OperationOutcome:
            rows = [
                {
                    "key": key,
                    "title": entry.fields.get("title"),
                    "doi": entry.fields.get("doi"),
                    "volume": entry.fields.get("volume"),
                }
                for key, entry in database.entries.items()
            ]
            return OperationOutcome(rows, len(rows))

        return _prepared(
            operation,
            _projection_contains(
                workload.records,
                required_fields=("key", "title", "doi"),
            ),
        )


def _parse_bibtex_string(text: str) -> Any:
    from pybtex.database.input import bibtex

    return bibtex.Parser().parse_string(text)


def _parsed_rows(database: Any) -> list[dict[str, Any]]:
    return [
        {
            "key": key,
            "type": entry.type,
            "title": entry.fields.get("title"),
            "year": entry.fields.get("year"),
            "doi": entry.fields.get("doi"),
            "volume": entry.fields.get("volume"),
            "pages": entry.fields.get("pages", "").replace("--", "-"),
            "container": entry.fields.get("journal", entry.fields.get("booktitle", "")),
            "authors": [
                [" ".join(person.last_names), " ".join(person.first_names + person.middle_names)]
                for person in entry.persons.get("author", [])
            ],
        }
        for key, entry in database.entries.items()
    ]
