from __future__ import annotations

import argparse
import csv
import json
import platform
import subprocess
import tempfile
from dataclasses import dataclass
from datetime import UTC, datetime
from importlib.metadata import PackageNotFoundError, version
from pathlib import Path
from time import perf_counter
from typing import Any, TypedDict

from benchmark.adapters import PackageAdapter, UnsupportedOperation, adapters
from benchmark.fixtures import SIZES, materialize_workload

NATIVE_ARTIFACT_NAMES = (
    "lib_native.dylib",
    "lib_native.so",
    "_native.dll",
    "_native.pyd",
)

RESULT_FIELDS = [
    "case",
    "group",
    "package",
    "package_version",
    "adapter_version",
    "phase",
    "operation_phase",
    "input",
    "input_size",
    "workload_family",
    "record_count",
    "input_bytes",
    "input_sha256",
    "source_format",
    "citation_count",
    "setup_included",
    "setup_seconds",
    "operation_count",
    "rounds",
    "warmups",
    "round",
    "seconds",
    "status",
    "detail",
    "python",
    "os",
    "cpu",
    "refkit_version",
    "refkit_commit",
    "build_mode",
]


Row = dict[str, object]


class Metadata(TypedDict):
    timestamp: str
    python: str
    os: str
    cpu: str
    refkit_commit: str
    build_mode: str
    packages: dict[str, str]


class SuiteResult(TypedDict):
    metadata: Metadata
    rows: list[Row]


@dataclass(frozen=True)
class CaseSpec:
    name: str
    group: str
    description: str


CASES: dict[str, CaseSpec] = {
    "bibtex_parse": CaseSpec("bibtex_parse", "parse", "Parse BibTeX into a package library model."),
    "bibtex_recovery_parse": CaseSpec(
        "bibtex_recovery_parse",
        "parse",
        "Parse dirty BibTeX and report whether valid records survive recovery.",
    ),
    "raw_bibtex_parse": CaseSpec(
        "raw_bibtex_parse",
        "raw",
        "Parse raw BibTeX while preserving raw document structure.",
    ),
    "raw_bibtex_write": CaseSpec(
        "raw_bibtex_write",
        "raw",
        "Write an already parsed raw BibTeX document after one field edit.",
    ),
    "raw_bibtex_roundtrip": CaseSpec(
        "raw_bibtex_roundtrip",
        "raw",
        "Parse raw BibTeX, edit one title, and write BibTeX text.",
    ),
    "style_load": CaseSpec("style_load", "setup", "Load the APA citation style."),
    "processor_setup": CaseSpec(
        "processor_setup",
        "setup",
        "Create a citation processor or document from prepared inputs.",
    ),
    "citation_render": CaseSpec("citation_render", "render", "Render one APA citation."),
    "bibliography_render": CaseSpec("bibliography_render", "render", "Render an APA bibliography."),
    "bibliography_seen_render": CaseSpec(
        "bibliography_seen_render",
        "render",
        "Render a bibliography for entries cited during the operation.",
    ),
    "repeated_render": CaseSpec(
        "repeated_render", "render", "Render repeated APA citations after setup."
    ),
    "rendered_text_access": CaseSpec(
        "rendered_text_access",
        "render-output",
        "Access text from an already rendered refkit citation.",
    ),
    "rendered_html_access": CaseSpec(
        "rendered_html_access",
        "render-output",
        "Access HTML from an already rendered refkit citation.",
    ),
    "rendered_tree_access": CaseSpec(
        "rendered_tree_access",
        "render-output",
        "Materialize the Python tree for an already rendered refkit citation.",
    ),
    "one_off_cite": CaseSpec(
        "one_off_cite",
        "one-off",
        "Read a BibTeX file, load a style, and render one APA citation.",
    ),
    "one_off_bibliography": CaseSpec(
        "one_off_bibliography",
        "one-off",
        "Read a BibTeX file, load a style, and render an APA bibliography.",
    ),
    "missing_reference": CaseSpec(
        "missing_reference", "error", "Resolve one missing citation key."
    ),
    "bulk_materialization": CaseSpec(
        "bulk_materialization",
        "materialize",
        "Materialize parsed entries into Python-visible key and title rows.",
    ),
    "library_keys": CaseSpec("library_keys", "inspect", "Enumerate all citation keys after setup."),
    "entry_lookup": CaseSpec(
        "entry_lookup", "inspect", "Look up a fixed set of entries after setup."
    ),
    "field_projection": CaseSpec(
        "field_projection",
        "inspect",
        "Project common scalar fields from all entries after setup.",
    ),
}


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    if args.list:
        print_case_list()
        return 0

    selected_cases = select_cases(args.case, args.group)
    selected_inputs = select_inputs(args.input)
    result = run_suite(
        case_names=selected_cases,
        input_sizes=selected_inputs,
        rounds=args.rounds,
        warmups=args.warmups,
        build_mode=args.build_mode,
    )

    if args.json:
        write_json(Path(args.json), result)
    if args.csv:
        write_csv(Path(args.csv), result["rows"])

    print_summary(result["rows"])
    return 1 if any(row["status"] == "failed" for row in result["rows"]) else 0


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Run refkit citation benchmarks.")
    parser.add_argument("--list", action="store_true", help="List benchmark cases and exit.")
    parser.add_argument("--case", action="append", choices=sorted(CASES), help="Run one case.")
    parser.add_argument(
        "--group", default="all", help="Run cases from a group. Use all for every case."
    )
    parser.add_argument(
        "--input", action="append", choices=[*SIZES, "all"], help="Input size to run."
    )
    parser.add_argument("--rounds", type=positive_int, default=5, help="Measured rounds per case.")
    parser.add_argument(
        "--warmups", type=non_negative_int, default=2, help="Warmup rounds per case."
    )
    parser.add_argument("--json", help="Write JSON results to this path.")
    parser.add_argument("--csv", help="Write CSV result rows to this path.")
    parser.add_argument(
        "--build-mode",
        choices=["auto", "debug", "release", "unknown"],
        default="auto",
        help="Recorded native build mode. auto inspects the local refkit extension.",
    )
    return parser.parse_args(argv)


def positive_int(value: str) -> int:
    parsed = int(value)
    if parsed <= 0:
        raise argparse.ArgumentTypeError("value must be greater than zero")
    return parsed


def non_negative_int(value: str) -> int:
    parsed = int(value)
    if parsed < 0:
        raise argparse.ArgumentTypeError("value must be zero or greater")
    return parsed


def select_cases(case_names: list[str] | None, group: str) -> list[str]:
    if case_names:
        return case_names
    if group == "all":
        return list(CASES)
    selected = [case.name for case in CASES.values() if case.group == group]
    if not selected:
        raise SystemExit(f"unknown benchmark group: {group}")
    return selected


def select_inputs(input_names: list[str] | None) -> list[str]:
    if not input_names:
        return list(SIZES)
    selected: list[str] = []
    for name in input_names:
        selected.extend(SIZES if name == "all" else [name])
    return list(dict.fromkeys(selected))


def run_suite(
    *,
    case_names: list[str],
    input_sizes: list[str],
    rounds: int,
    warmups: int,
    build_mode: str = "auto",
    package_adapters: list[PackageAdapter] | None = None,
) -> SuiteResult:
    suite_adapters = package_adapters if package_adapters is not None else adapters()
    metadata = machine_metadata(build_mode)
    rows: list[Row] = []

    with tempfile.TemporaryDirectory(prefix="refkit-benchmark-") as tmp:
        root = Path(tmp)
        for input_size in input_sizes:
            workload = materialize_workload(input_size, root)
            for case_name in case_names:
                case = CASES[case_name]
                for adapter in suite_adapters:
                    rows.extend(
                        run_adapter_case(
                            adapter=adapter,
                            case=case,
                            workload=workload,
                            directory=root,
                            rounds=rounds,
                            warmups=warmups,
                            metadata=metadata,
                        )
                    )

    return {"metadata": metadata, "rows": rows}


def run_adapter_case(
    *,
    adapter: PackageAdapter,
    case: CaseSpec,
    workload: Any,
    directory: Path,
    rounds: int,
    warmups: int,
    metadata: Metadata,
) -> list[Row]:
    base = base_row(adapter, case, workload, metadata, rounds, warmups)
    setup_start = perf_counter()
    try:
        prepared = adapter.prepare(case.name, workload, directory)
    except UnsupportedOperation as exc:
        setup_seconds = perf_counter() - setup_start
        return [
            {
                **base,
                "phase": "unsupported",
                "operation_phase": "unsupported",
                "source_format": "unsupported",
                "input_bytes": 0,
                "input_sha256": "",
                "citation_count": 0,
                "setup_included": False,
                "setup_seconds": setup_seconds,
                "operation_count": 0,
                "round": 0,
                "seconds": 0.0,
                "status": "unsupported",
                "detail": exc.reason,
            }
        ]
    except Exception as exc:
        setup_seconds = perf_counter() - setup_start
        return [
            {
                **base,
                "phase": "setup",
                "operation_phase": "setup",
                "source_format": "unknown",
                "input_bytes": 0,
                "input_sha256": "",
                "citation_count": 0,
                "setup_included": True,
                "setup_seconds": setup_seconds,
                "operation_count": 0,
                "round": 0,
                "seconds": 0.0,
                "status": "failed",
                "detail": repr(exc),
            }
        ]
    setup_seconds = perf_counter() - setup_start
    operation_fields = prepared_row_fields(prepared, workload, setup_seconds)

    for _ in range(warmups):
        try:
            outcome = prepared.operation()
            prepared.check(outcome)
        except Exception as exc:
            return [
                {
                    **base,
                    **operation_fields,
                    "phase": prepared.phase,
                    "round": 0,
                    "seconds": 0.0,
                    "status": "failed",
                    "detail": repr(exc),
                }
            ]

    rows: list[Row] = []
    for round_index in range(1, rounds + 1):
        start = perf_counter()
        try:
            outcome = prepared.operation()
            seconds = perf_counter() - start
            prepared.check(outcome)
            rows.append(
                {
                    **base,
                    **operation_fields,
                    "phase": prepared.phase,
                    "round": round_index,
                    "seconds": seconds,
                    "status": "ok",
                    "detail": outcome.detail,
                    "operation_count": outcome.count,
                }
            )
        except Exception as exc:
            seconds = perf_counter() - start
            rows.append(
                {
                    **base,
                    **operation_fields,
                    "phase": prepared.phase,
                    "round": round_index,
                    "seconds": seconds,
                    "status": "failed",
                    "detail": repr(exc),
                }
            )
            break
    return rows


def base_row(
    adapter: PackageAdapter,
    case: CaseSpec,
    workload: Any,
    metadata: Metadata,
    rounds: int,
    warmups: int,
) -> Row:
    adapter_version = package_version(adapter.distribution)
    return {
        "case": case.name,
        "group": case.group,
        "package": adapter.name,
        "package_version": adapter_version,
        "adapter_version": adapter_version,
        "input": workload.size,
        "input_size": workload.size,
        "workload_family": workload.family,
        "record_count": workload.record_count,
        "rounds": rounds,
        "warmups": warmups,
        "python": metadata["python"],
        "os": metadata["os"],
        "cpu": metadata["cpu"],
        "refkit_version": metadata["packages"].get("refkit", "unknown"),
        "refkit_commit": metadata["refkit_commit"],
        "build_mode": metadata["build_mode"],
    }


def prepared_row_fields(
    prepared: Any,
    workload: Any,
    setup_seconds: float,
) -> Row:
    source_format = str(prepared.metadata.get("source_format", "unknown"))
    return {
        "operation_phase": prepared.phase,
        "input_bytes": workload.source_byte_count(source_format),
        "input_sha256": workload.source_sha256(source_format),
        "source_format": source_format,
        "citation_count": int(prepared.metadata.get("citation_count", 0)),
        "setup_included": bool(prepared.metadata.get("setup_included", False)),
        "setup_seconds": setup_seconds,
        "operation_count": 0,
    }


def machine_metadata(build_mode: str = "auto") -> Metadata:
    return {
        "timestamp": datetime.now(UTC).isoformat(),
        "python": platform.python_version(),
        "os": platform.platform(),
        "cpu": platform.processor() or platform.machine() or "unknown",
        "refkit_commit": refkit_commit(),
        "build_mode": detect_build_mode() if build_mode == "auto" else build_mode,
        "packages": {
            "refkit": package_version("refkit"),
            "citeproc-py": package_version("citeproc-py"),
            "bibtexparser": package_version("bibtexparser"),
            "citeproc-py-styles": package_version("citeproc-py-styles"),
        },
    }


def package_version(distribution: str) -> str:
    try:
        return version(distribution)
    except PackageNotFoundError:
        return "not-installed"


def refkit_commit() -> str:
    try:
        completed = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        )
    except Exception:
        return "unknown"
    return completed.stdout.strip() or "unknown"


def detect_build_mode(native_file: str | None = None, artifact_root: Path | None = None) -> str:
    if native_file is None:
        try:
            import refkit._native as native
        except Exception:  # pragma: no cover
            return "unknown"
        native_file = getattr(native, "__file__", "")

    native_path = Path(native_file)
    parts = set(native_path.parts)
    if "release" in parts:
        return "release"
    if "debug" in parts:
        return "debug"

    root = artifact_root if artifact_root is not None else Path(".")
    release_artifacts = _native_artifact_candidates(root, "release")
    debug_artifacts = _native_artifact_candidates(root, "debug")
    if any(_same_artifact(native_path, artifact) for artifact in release_artifacts):
        return "release"
    if any(_same_artifact(native_path, artifact) for artifact in debug_artifacts):
        return "debug"
    return "unknown"


def _native_artifact_candidates(root: Path, profile: str) -> list[Path]:
    directory = root / "target" / profile
    return [directory / name for name in NATIVE_ARTIFACT_NAMES]


def _same_artifact(left: Path, right: Path) -> bool:
    if not left.exists() or not right.exists():
        return False
    left_stat = left.stat()
    right_stat = right.stat()
    return left_stat.st_size == right_stat.st_size and left_stat.st_mtime == right_stat.st_mtime


def print_case_list() -> None:
    for case in CASES.values():
        print(f"{case.name}\t{case.group}\t{case.description}")


def write_json(path: Path, result: SuiteResult) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_csv(path: Path, rows: list[Row]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=RESULT_FIELDS)
        writer.writeheader()
        writer.writerows({field: row.get(field, "") for field in RESULT_FIELDS} for row in rows)


def print_summary(rows: list[Row]) -> None:
    counts: dict[str, int] = {}
    for row in rows:
        counts[str(row["status"])] = counts.get(str(row["status"]), 0) + 1
    print(json.dumps({"rows": len(rows), "status": counts}, sort_keys=True))


if __name__ == "__main__":  # pragma: no cover
    raise SystemExit(main())
