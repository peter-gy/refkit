from __future__ import annotations

import importlib
import json
import re
from collections.abc import Callable, Iterable, Mapping
from dataclasses import dataclass, field
from functools import cached_property
from hashlib import sha256
from importlib.metadata import PackageNotFoundError, distribution
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

    @cached_property
    def artifact_metadata(self) -> dict[str, str]:
        result = {
            "artifact_path": "unknown",
            "artifact_sha256": "unknown",
            "artifact_source_url": "unknown",
            "artifact_source_revision": "unknown",
            "distribution_record_sha256": "unknown",
            "build_mode": "unknown",
        }
        try:
            installed = distribution(self.distribution)
        except PackageNotFoundError:
            return result
        record = installed.read_text("RECORD")
        if record is not None:
            result["distribution_record_sha256"] = sha256(record.encode()).hexdigest()
        direct_url = installed.read_text("direct_url.json")
        if direct_url is not None:
            source = json.loads(direct_url)
            result["artifact_source_url"] = source.get("url", "unknown")
            result["artifact_source_revision"] = source.get("vcs_info", {}).get(
                "commit_id", "unknown"
            )
        module_name = {
            "refkit": "refkit._native",
            "polars-refkit": "polars_refkit._internal",
            "citeproc-py": "citeproc",
        }.get(self.distribution, self.distribution.replace("-", "_"))
        try:
            module = importlib.import_module(module_name)
        except ImportError:
            return result
        module_path = getattr(module, "__file__", None)
        if module_path is not None:
            path = Path(module_path)
            result["artifact_path"] = str(path.resolve())
            result["artifact_sha256"] = sha256(path.read_bytes()).hexdigest()
        if self.distribution in {"refkit", "polars-refkit"}:
            build_mode = getattr(module, "build_mode", "unknown")
            if build_mode in {"debug", "release"}:
                result["build_mode"] = build_mode
        else:
            result["build_mode"] = "python"
        return result


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
    def tokens(text: str) -> tuple[str, ...]:
        return tuple(re.findall(r"\w+", text.replace("https://doi.org/", "")))

    expected = []
    for record in records:
        authors = []
        for family, given in record.authors or ((record.family, record.given),):
            initials = " ".join(part[0] for part in re.split(r"[-\s]+", given) if part)
            authors.append(f"{family} {initials}")
        expected.append(
            tokens(
                " ".join(
                    [
                        *authors,
                        str(record.year),
                        record.title,
                        record.container,
                        "" if record.volume is None else str(record.volume),
                        record.page_range,
                        record.doi or "",
                    ]
                )
            )
        )

    def check(outcome: OperationOutcome) -> None:
        rows = _non_empty_lines(str(outcome.value))
        if len(rows) != len(records):
            raise AssertionError(f"expected {len(records)} bibliography rows, got {len(rows)}")
        actual = [tokens(row) for row in rows]
        if sorted(actual) != sorted(expected):
            raise AssertionError(
                "expected complete bibliography fields and ordered author initials"
            )

    return check


def _non_empty_lines(value: str) -> list[str]:
    return [line.strip() for line in value.splitlines() if line.strip()]


def _detail_contains(needle: str) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        if needle not in outcome.detail:
            raise AssertionError(f"expected detail to contain {needle!r}")

    return check


def _raw_roundtrip_check(workload: Workload) -> Callable[[OperationOutcome], None]:
    import bibtexparser

    original = bibtexparser.parse_string(workload.raw_bibtex)
    expected = [
        (entry.key, entry.entry_type, [(field.key, field.value) for field in entry.fields])
        for entry in original.entries
    ]
    key, entry_type, fields = expected[0]
    expected[0] = (
        key,
        entry_type,
        [(name, "Edited Benchmark Title" if name == "title" else value) for name, value in fields],
    )

    def check(outcome: OperationOutcome) -> None:
        text = Path(str(outcome.value)).read_text(encoding="utf-8")
        written = bibtexparser.parse_string(text)
        actual = [
            (entry.key, entry.entry_type, [(field.key, field.value) for field in entry.fields])
            for entry in written.entries
        ]
        if written.failed_blocks or actual != expected:
            raise AssertionError(
                "expected the first title edit and all other entry fields preserved"
            )
        if any(term not in text for term in workload.raw_preservation_terms):
            raise AssertionError("expected the raw document blocks to survive writeback")

    return check


def _parsed_records_match(
    records: tuple[Any, ...],
    extract: Callable[[Any], Iterable[Mapping[str, Any]]],
) -> Callable[[OperationOutcome], None]:
    def check(outcome: OperationOutcome) -> None:
        rows = list(extract(outcome.value))
        _entries_match(records)(OperationOutcome(rows, len(rows)))
        for row, record in zip(rows, records, strict=True):
            expected = {
                "author": " and ".join(
                    f"{family}, {given}" if given else family
                    for family, given in record.authors or ((record.family, record.given),)
                ),
                "doi": record.doi,
                "volume": None if record.volume is None else str(record.volume),
                "year": str(record.year),
                "pages": record.page_range.replace("--", "-"),
                "container": record.container,
                "authors": [
                    list(author) for author in (record.authors or ((record.family, record.given),))
                ],
                "type": "inproceedings" if record.item_type == "paper-conference" else "article",
            }
            for name, value in expected.items():
                if name in row and row[name] != value:
                    raise AssertionError(
                        f"expected {name} for {record.key!r}: {value!r}, got {row[name]!r}"
                    )

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
