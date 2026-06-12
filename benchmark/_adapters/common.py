from __future__ import annotations

from collections.abc import Callable, Iterable, Mapping
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, cast

from benchmark.fixtures import Workload

OutcomeValue = object


def _noop_cleanup() -> None:
    return None


class UnsupportedOperation(Exception):
    def __init__(self, reason: str) -> None:
        super().__init__(reason)
        self.reason = reason


@dataclass(frozen=True)
class OperationOutcome:
    value: OutcomeValue
    count: int
    detail: str = ""
    seconds: float | None = None


@dataclass(frozen=True)
class PreparedOperation:
    phase: str
    operation: Callable[[], OperationOutcome]
    check: Callable[[OperationOutcome], None]
    metadata: dict[str, object] = field(default_factory=dict)
    cleanup: Callable[[], None] = _noop_cleanup


class PackageAdapter:
    name: str
    distribution: str

    def prepare(self, case: str, workload: Workload, directory: Path) -> PreparedOperation:
        method_name = f"prepare_{case}"
        method = getattr(self, method_name, None)
        if method is None:
            raise UnsupportedOperation(f"{self.name} does not support {case}")
        return method(workload, directory)

    def version(self) -> str | None:
        return None


def _prepared(
    phase: str,
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
        phase=phase,
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


def _recovery_parse_result(expected: int) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        if outcome.count == expected:
            return
        raise AssertionError(f"expected {expected} recovered entries, got {outcome.count}")

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
