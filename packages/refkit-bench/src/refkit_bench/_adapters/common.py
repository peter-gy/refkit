from __future__ import annotations

import re
from collections.abc import Callable, Iterable, Mapping
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, cast

from refkit_bench.fixtures import Workload

OutcomeValue = object


def _noop_cleanup() -> None:
    return None


class MissingBenchmarkOperation(Exception):
    def __init__(self, reason: str) -> None:
        super().__init__(reason)
        self.reason = reason


@dataclass(frozen=True)
class OperationOutcome:
    value: OutcomeValue
    count: int
    detail: str = ""
    seconds: float | None = None
    metadata: Mapping[str, object] = field(default_factory=dict)


@dataclass(frozen=True)
class PreparedOperation:
    operation: Callable[[], OperationOutcome]
    check: Callable[[OperationOutcome], None]
    metadata: dict[str, object] = field(default_factory=dict)
    cleanup: Callable[[], None] = _noop_cleanup


class PackageAdapter:
    name: str
    distribution: str

    def prepare(self, operation: str, workload: Workload, directory: Path) -> PreparedOperation:
        method_name = f"prepare_{operation}"
        method = getattr(self, method_name, None)
        if method is None:
            raise MissingBenchmarkOperation(f"{self.name} has no benchmark operation {operation}")
        return method(workload, directory)

    def version(self) -> str | None:
        return None


def _prepared(
    operation: Callable[[], OperationOutcome],
    check: Callable[[OperationOutcome], None],
    *,
    source_format: str = "bibtex",
    setup_included: bool = False,
    citation_count: int = 0,
    execution_mode: str = "",
    cleanup: Callable[[], None] = _noop_cleanup,
) -> PreparedOperation:
    return PreparedOperation(
        operation=operation,
        check=check,
        metadata={
            "source_format": source_format,
            "setup_included": setup_included,
            "citation_count": citation_count,
            "execution_mode": execution_mode,
        },
        cleanup=cleanup,
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
            if not _title_matches(row.get("title"), record):
                raise AssertionError(f"expected title for {record.key!r}")
            doi = row.get("doi")
            expected_doi = getattr(record, "doi", None)
            if ("doi" in required_fields and doi is None) or (
                doi is not None and expected_doi and str(doi) != str(expected_doi)
            ):
                raise AssertionError(f"expected DOI for {record.key!r}")
            volume = row.get("volume")
            expected_volume = getattr(record, "volume", None)
            if ("volume" in required_fields and volume is None) or (
                volume is not None
                and expected_volume is not None
                and str(volume) != str(expected_volume)
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
            if not _title_matches(_first(title), record):
                raise AssertionError(f"expected title for {record.key!r}")

    return check


def _text_contains(needle: str) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        if needle not in str(outcome.value):
            raise AssertionError(f"expected output to contain {needle!r}")

    return check


def _citation_output_matches(records: tuple[Any, ...]) -> Callable[[OperationOutcome], None]:
    expected = [record.citation_text or f"({record.family}, {record.year})" for record in records]

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
        normalized_rows = [row.replace("\u2013", "-") for row in rows]
        for record in records:
            page_range = getattr(record, "page_range", "")
            expected_values = list(
                record.bibliography_terms
                or (
                    record.family,
                    str(record.year),
                    record.title,
                    getattr(record, "container", ""),
                    str(record.volume) if record.volume is not None else "",
                    page_range,
                    record.doi or "",
                )
            )
            expected_values = [value.replace("\u2013", "-") for value in expected_values if value]
            if not any(all(value in row for value in expected_values) for row in normalized_rows):
                raise AssertionError(
                    f"expected bibliography to contain row terms for {record.key!r}"
                )

    return check


def _non_empty_lines(value: str) -> list[str]:
    return [line.strip() for line in value.splitlines() if line.strip()]


def _detail_contains(needle: str) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        if needle not in outcome.detail:
            raise AssertionError(f"expected detail to contain {needle!r}")

    return check


def _raw_roundtrip_check(
    keys: list[str],
    preservation_terms: tuple[str, ...] = (),
) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        path = Path(str(outcome.value))
        text = path.read_text(encoding="utf-8")
        expected = [
            "Edited Benchmark Title",
            *keys,
            *preservation_terms,
        ]
        missing = [value for value in expected if value not in text]
        if missing:
            raise AssertionError(f"expected written file to contain {missing[0]!r}")

    return check


def _raw_blocks_cover(workload: Workload) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        rows = list(cast(Iterable[Mapping[str, Any]], outcome.value))
        if outcome.count != len(rows):
            raise AssertionError(f"expected raw block count {len(rows)}, got {outcome.count}")
        kinds = {str(row.get("kind")) for row in rows}
        entry_keys = {str(row.get("key")) for row in rows if row.get("kind") == "entry"}
        missing_keys = [key for key in workload.keys if key not in entry_keys]
        if missing_keys:
            raise AssertionError(f"expected raw blocks to include entry {missing_keys[0]!r}")
        if "comment" not in kinds:
            raise AssertionError("expected raw blocks to include a comment")
        if workload.family == "synthetic_scale":
            for kind in ("string", "preamble"):
                if kind not in kinds:
                    raise AssertionError(f"expected raw blocks to include {kind!r}")

    return check


def _duplicate_signals_cover(workload: Workload) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        rows = list(cast(Iterable[Mapping[str, Any]], outcome.value))
        if outcome.count != len(rows):
            raise AssertionError(
                f"expected duplicate signal count {len(rows)}, got {outcome.count}"
            )
        expected = {
            ("duplicate_entry", workload.duplicate_entry_key, ""),
            ("duplicate_field", workload.duplicate_field_key, workload.duplicate_field_name),
        }
        actual = {
            (
                str(row.get("kind", "")),
                str(row.get("key", "")),
                str(row.get("field", "")),
            )
            for row in rows
        }
        missing = expected - actual
        if missing:
            kind, key, field = sorted(missing)[0]
            label = f"{key}.{field}" if field else key
            raise AssertionError(f"expected {kind} signal for {label!r}")

    return check


def _lookup_keys(workload: Workload) -> list[str]:
    return workload.keys[: min(16, len(workload.keys))]


def _first(value: object) -> object:
    if isinstance(value, list):
        return value[0] if value else None
    return value


def _title_matches(value: object, record: Any) -> bool:
    title = str(value)
    expected = {record.title}
    raw_title = getattr(record, "raw_title", "")
    if raw_title:
        expected.add(raw_title)
    normalized = _bibtex_visible_title(title)
    return any(
        title == candidate or normalized == _bibtex_visible_title(candidate)
        for candidate in expected
    )


def _bibtex_visible_title(value: object) -> str:
    visible = str(value).replace("{", "").replace("}", "")
    return re.sub(r"\s+", " ", visible).strip()
