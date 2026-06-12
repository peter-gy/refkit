from __future__ import annotations

import contextlib
import io
from collections.abc import Iterable
from importlib.metadata import version as package_version
from pathlib import Path
from time import perf_counter_ns
from typing import Any

from refkit_bench._adapters.common import (
    OperationOutcome,
    PackageAdapter,
    PreparedOperation,
    _all_checks,
    _count_is,
    _duplicate_signals_cover,
    _entries_match,
    _keys_are,
    _lookup_keys,
    _prepared,
    _projection_contains,
    _raw_blocks_cover,
    _raw_roundtrip_check,
)
from refkit_bench.fixtures import Workload

BIBTEXPARSER_V2_VERSION = "2.0.0b9"


class BibtexparserV2Adapter(PackageAdapter):
    name = "bibtexparser-2.x"
    distribution = "bibtexparser"

    def prepare_parse_bibtex(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        _require_bibtexparser_v2()

        def operation() -> OperationOutcome:
            database = bibtexparser.parse_file(str(workload.bibtex_path))
            return OperationOutcome(database, len(database.entries))

        return _prepared(operation, _count_is(len(workload.records)), setup_included=True)

    def prepare_parse_bibtex_text(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        _require_bibtexparser_v2()

        def operation() -> OperationOutcome:
            database = bibtexparser.parse_string(workload.bibtex)
            return OperationOutcome(database, len(database.entries))

        return _prepared(operation, _count_is(len(workload.records)), setup_included=True)

    def prepare_recover_dirty_bibtex(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import bibtexparser

        _require_bibtexparser_v2()

        expected_signatures = (
            []
            if workload.dirty_bibtex == workload.bibtex
            else [
                "ParsingFailedBlock:missing",
                f"DuplicateBlockKeyBlock:{workload.keys[0]}",
            ]
        )

        def operation() -> OperationOutcome:
            with contextlib.redirect_stderr(io.StringIO()):
                database = bibtexparser.parse_file(str(workload.dirty_bibtex_path))
            failed_blocks = getattr(database, "failed_blocks", ())
            failed = len(failed_blocks)
            signatures = _bibtexparser_v2_failed_signatures(failed_blocks)
            return OperationOutcome(
                database,
                len(database.entries),
                f"failed_blocks={failed};failed_signatures={signatures}",
                metadata={"failed_block_count": failed, "diagnostic_count": failed},
            )

        return _prepared(
            operation,
            _all_checks(
                _count_is(len(workload.records)),
                _bibtexparser_v2_recovery_matches(expected_signatures),
            ),
            source_format="dirty_bibtex",
            setup_included=True,
        )

    def prepare_extract_diagnostics(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        _require_bibtexparser_v2()
        expected = 0 if workload.dirty_bibtex == workload.bibtex else 2

        def operation() -> OperationOutcome:
            with contextlib.redirect_stderr(io.StringIO()):
                database = bibtexparser.parse_file(str(workload.dirty_bibtex_path))
            failed_blocks = getattr(database, "failed_blocks", ())
            rows = [
                {
                    "kind": type(block).__name__,
                    "key": getattr(block, "key", None)
                    or _bibtexparser_block_key(getattr(block, "raw", "")),
                }
                for block in failed_blocks
            ]
            return OperationOutcome(
                rows,
                len(rows),
                _bibtexparser_v2_failed_signatures(failed_blocks),
                metadata={"failed_block_count": len(rows), "diagnostic_count": len(rows)},
            )

        return _prepared(
            operation,
            _count_is(expected),
            source_format="dirty_bibtex",
            setup_included=True,
        )

    def prepare_parse_raw_bibtex(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        _require_bibtexparser_v2()

        def operation() -> OperationOutcome:
            database = bibtexparser.parse_string(workload.raw_bibtex)
            return OperationOutcome(database, len(database.entries))

        return _prepared(
            operation,
            _count_is(len(workload.records)),
            source_format="raw_bibtex",
            setup_included=True,
        )

    def prepare_materialize_raw_blocks(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import bibtexparser

        _require_bibtexparser_v2()
        database = bibtexparser.parse_string(workload.raw_bibtex)

        def operation() -> OperationOutcome:
            rows = [
                {
                    "kind": _bibtexparser_block_kind(block),
                    "key": getattr(block, "key", "") or "",
                    "raw_bytes": len(str(getattr(block, "raw", "")).encode("utf-8")),
                }
                for block in database.blocks
            ]
            return OperationOutcome(rows, len(rows))

        return _prepared(
            operation,
            _raw_blocks_cover(workload),
            source_format="raw_bibtex",
        )

    def prepare_handle_duplicates(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        _require_bibtexparser_v2()

        def operation() -> OperationOutcome:
            with contextlib.redirect_stderr(io.StringIO()):
                database = bibtexparser.parse_string(workload.duplicate_bibtex)
            rows = []
            for block in getattr(database, "failed_blocks", ()):
                block_type = type(block).__name__
                key = getattr(block, "key", None) or _bibtexparser_block_key(
                    getattr(block, "raw", "")
                )
                if block_type == "DuplicateBlockKeyBlock":
                    rows.append(
                        {
                            "kind": "duplicate_entry",
                            "key": key,
                            "field": "",
                            "count": 2,
                        }
                    )
                elif block_type == "DuplicateFieldKeyBlock":
                    rows.append(
                        {
                            "kind": "duplicate_field",
                            "key": key,
                            "field": workload.duplicate_field_name,
                            "count": 2,
                        }
                    )
            return OperationOutcome(
                rows,
                len(rows),
                _bibtexparser_v2_failed_signatures(getattr(database, "failed_blocks", ())),
                metadata={"failed_block_count": len(rows), "diagnostic_count": len(rows)},
            )

        return _prepared(
            operation,
            _duplicate_signals_cover(workload),
            source_format="duplicate_bibtex",
            setup_included=True,
        )

    def prepare_write_edited_raw_bibtex(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import bibtexparser

        _require_bibtexparser_v2()
        database = bibtexparser.parse_string(workload.raw_bibtex)

        def operation() -> OperationOutcome:
            _bibtexparser_v2_set_field(database.entries[0], "title", "Edited Benchmark Title")
            text = bibtexparser.write_string(database)
            path = directory / f"bibtexparser-v2-raw-write-{perf_counter_ns()}.bib"
            path.write_text(text, encoding="utf-8")
            return OperationOutcome(path, len(database.entries), path.name)

        return _prepared(
            operation,
            _raw_roundtrip_check(workload.keys, workload.raw_preservation_terms),
            source_format="raw_bibtex",
        )

    def prepare_roundtrip_raw_bibtex_edit(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import bibtexparser

        _require_bibtexparser_v2()

        def operation() -> OperationOutcome:
            database = bibtexparser.parse_string(workload.raw_bibtex)
            _bibtexparser_v2_set_field(database.entries[0], "title", "Edited Benchmark Title")
            text = bibtexparser.write_string(database)
            path = directory / f"bibtexparser-v2-raw-{perf_counter_ns()}.bib"
            path.write_text(text, encoding="utf-8")
            return OperationOutcome(path, len(database.entries), path.name)

        return _prepared(
            operation,
            _raw_roundtrip_check(workload.keys, workload.raw_preservation_terms),
            source_format="raw_bibtex",
            setup_included=True,
        )

    def prepare_materialize_entry_rows(
        self, workload: Workload, directory: Path
    ) -> PreparedOperation:
        import bibtexparser

        _require_bibtexparser_v2()
        database = bibtexparser.parse_string(workload.bibtex)

        def operation() -> OperationOutcome:
            rows = [
                {"key": entry.key, "title": _bibtexparser_v2_field_value(entry, "title")}
                for entry in database.entries
            ]
            return OperationOutcome(rows, len(rows))

        return _prepared(
            operation,
            _projection_contains(workload.records, required_fields=("key", "title")),
        )

    def prepare_list_keys(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        _require_bibtexparser_v2()
        database = bibtexparser.parse_string(workload.bibtex)

        def operation() -> OperationOutcome:
            keys = [entry.key for entry in database.entries]
            return OperationOutcome(keys, len(keys))

        return _prepared(operation, _keys_are(workload.keys))

    def prepare_lookup_entries(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        _require_bibtexparser_v2()
        database = bibtexparser.parse_string(workload.bibtex)
        entries = {entry.key: entry for entry in database.entries}
        keys = _lookup_keys(workload)

        def operation() -> OperationOutcome:
            rows = [
                {
                    "key": key,
                    "title": _bibtexparser_v2_field_value(entries[key], "title"),
                }
                for key in keys
            ]
            return OperationOutcome(rows, len(rows))

        return _prepared(operation, _entries_match(workload.records[: len(keys)]))

    def prepare_project_fields(self, workload: Workload, directory: Path) -> PreparedOperation:
        import bibtexparser

        _require_bibtexparser_v2()
        database = bibtexparser.parse_string(workload.bibtex)

        def operation() -> OperationOutcome:
            rows = [
                {
                    "key": entry.key,
                    "title": _bibtexparser_v2_field_value(entry, "title"),
                    "doi": _bibtexparser_v2_field_value(entry, "doi"),
                    "volume": _bibtexparser_v2_field_value(entry, "volume"),
                }
                for entry in database.entries
            ]
            return OperationOutcome(rows, len(rows))

        return _prepared(
            operation,
            _projection_contains(
                workload.records,
                required_fields=("key", "title", "doi"),
            ),
        )


def _bibtexparser_v2_field_value(entry: Any, key: str) -> Any:
    field = entry.fields_dict.get(key)
    if field is None:
        return None
    return field.value


def _require_bibtexparser_v2() -> None:
    installed = package_version("bibtexparser")
    if installed != BIBTEXPARSER_V2_VERSION:
        raise RuntimeError(
            "bibtexparser-2.x benchmark requires "
            f"bibtexparser=={BIBTEXPARSER_V2_VERSION}, found {installed}"
        )


def _bibtexparser_v2_set_field(entry: Any, key: str, value: str) -> None:
    field = entry.fields_dict.get(key)
    if field is None:
        from bibtexparser.model import Field

        entry.set_field(Field(key, value))
        return
    field.value = value


def _bibtexparser_v2_failed_signatures(blocks: Iterable[Any]) -> str:
    signatures = []
    for block in blocks:
        key = getattr(block, "key", None)
        if key is None:
            key = _bibtexparser_block_key(getattr(block, "raw", ""))
        signatures.append(f"{type(block).__name__}:{key or 'unknown'}")
    return ",".join(signatures)


def _bibtexparser_block_kind(block: Any) -> str:
    name = type(block).__name__
    if name in {"ImplicitComment", "ExplicitComment"}:
        return "comment"
    if name == "String":
        return "string"
    if name == "Preamble":
        return "preamble"
    if name == "Entry":
        return "entry"
    return "failed"


def _bibtexparser_v2_recovery_matches(
    expected_signatures: list[str],
) -> Any:
    def check(outcome: OperationOutcome) -> None:
        failed_blocks = tuple(getattr(outcome.value, "failed_blocks", ()))
        signatures = _bibtexparser_v2_failed_signatures(failed_blocks)
        actual = [] if signatures == "" else signatures.split(",")
        if actual != expected_signatures:
            raise AssertionError(
                f"expected failed block signatures {expected_signatures!r}, got {actual!r}"
            )

    return check


def _bibtexparser_block_key(raw: object) -> str | None:
    after_open = str(raw).partition("{")[2]
    return after_open.split(",", 1)[0].strip() or None
