from __future__ import annotations

import argparse
import csv
import json
import platform
import subprocess
import sys
import tempfile
from dataclasses import dataclass
from datetime import UTC, datetime
from importlib.metadata import PackageNotFoundError, version
from pathlib import Path
from time import perf_counter
from typing import Any, TypedDict

from refkit_bench.adapters import (
    PackageAdapter,
    adapters,
)
from refkit_bench.fixtures import SIZES, materialize_workload

NATIVE_ARTIFACT_NAMES = (
    "lib_native.dylib",
    "lib_native.so",
    "_native.dll",
    "_native.pyd",
)

RESULT_FIELDS = [
    "lane",
    "group",
    "capability",
    "workflow",
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
    "execution_mode",
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
class LaneParticipant:
    package: str
    adapter_operation: str


@dataclass(frozen=True)
class LaneSpec:
    name: str
    group: str
    capability: str
    workflow: str
    phase: str
    participants: tuple[LaneParticipant, ...]
    description: str


REFKIT = "refkit"
POLARS_REFKIT = "polars-refkit"
CITEPROC_PY = "citeproc-py"
BIBTEXPARSER_V2 = "bibtexparser-2.x"


def participant(package: str, adapter_operation: str) -> LaneParticipant:
    return LaneParticipant(package, adapter_operation)


def participants(adapter_operation: str, *packages: str) -> tuple[LaneParticipant, ...]:
    return tuple(participant(package, adapter_operation) for package in packages)


LANES: dict[str, LaneSpec] = {
    "input.bibtex": LaneSpec(
        "input.bibtex",
        "input.normalized",
        "normalized_bibliography_input",
        "bibtex_input",
        "input",
        (
            participant(REFKIT, "parse_bibtex"),
            participant(POLARS_REFKIT, "parse_bibtex"),
            participant(POLARS_REFKIT, "parse_bibtex_lazy"),
            participant(BIBTEXPARSER_V2, "parse_bibtex"),
        ),
        "Parse clean BibTeX into the interface's normalized or queryable model.",
    ),
    "input.dirty-bibtex": LaneSpec(
        "input.dirty-bibtex",
        "input.normalized",
        "normalized_bibliography_input",
        "dirty_bibtex_recovery",
        "input-recovery",
        participants("recover_dirty_bibtex", REFKIT, BIBTEXPARSER_V2),
        "Recover valid entries from dirty BibTeX and report parse diagnostics.",
    ),
    "raw-bibtex.parse": LaneSpec(
        "raw-bibtex.parse",
        "raw-bibtex",
        "raw_bibtex_document",
        "raw_document_read",
        "raw-read",
        participants("parse_raw_bibtex", REFKIT, BIBTEXPARSER_V2),
        "Parse raw BibTeX while preserving document structure.",
    ),
    "raw-bibtex.write": LaneSpec(
        "raw-bibtex.write",
        "raw-bibtex",
        "raw_bibtex_document",
        "raw_document_write",
        "raw-write",
        participants("write_edited_raw_bibtex", REFKIT, BIBTEXPARSER_V2),
        "Write an already parsed raw BibTeX document after one field edit.",
    ),
    "raw-bibtex.roundtrip": LaneSpec(
        "raw-bibtex.roundtrip",
        "raw-bibtex",
        "raw_bibtex_document",
        "raw_document_edit_roundtrip",
        "raw-roundtrip",
        participants("roundtrip_raw_bibtex_edit", REFKIT, BIBTEXPARSER_V2),
        "Parse raw BibTeX, edit one title, and write BibTeX text.",
    ),
    "style.load": LaneSpec(
        "style.load",
        "style",
        "citation_style_input",
        "style_resolution",
        "style-resolution",
        participants("load_bundled_style", REFKIT, CITEPROC_PY),
        "Resolve the APA citation style after benchmark warmup.",
    ),
    "style.processor-setup": LaneSpec(
        "style.processor-setup",
        "style",
        "citation_style_input",
        "processor_creation",
        "processor-setup",
        participants("create_processor", REFKIT, CITEPROC_PY),
        "Create a citation processor or document from prepared inputs.",
    ),
    "render.prepared-citation": LaneSpec(
        "render.prepared-citation",
        "render.prepared",
        "citation_rendering",
        "prepared_citation",
        "render",
        participants("render_one_prepared_citation", REFKIT, CITEPROC_PY),
        "Render one APA citation from prepared citation data.",
    ),
    "render.prepared-bibliography": LaneSpec(
        "render.prepared-bibliography",
        "render.prepared",
        "bibliography_rendering",
        "prepared_bibliography",
        "render",
        participants("render_prepared_bibliography", REFKIT, CITEPROC_PY),
        "Render one APA bibliography from prepared citation data.",
    ),
    "render.cited-bibliography": LaneSpec(
        "render.cited-bibliography",
        "render.prepared",
        "bibliography_rendering",
        "cited_bibliography",
        "render",
        participants("render_cited_bibliography", REFKIT, CITEPROC_PY),
        "Cite every entry during the operation, then render the cited bibliography.",
    ),
    "render.repeated-citations": LaneSpec(
        "render.repeated-citations",
        "render.prepared",
        "citation_rendering",
        "citation_sequence",
        "render",
        participants("render_repeated_citations", REFKIT, CITEPROC_PY),
        "Render repeated APA citations from prepared citation data.",
    ),
    "render.output-text": LaneSpec(
        "render.output-text",
        "render.output",
        "rendered_output",
        "rendered_text_access",
        "output-access",
        participants("access_rendered_text", REFKIT),
        "Access text from an already rendered refkit citation.",
    ),
    "render.output-html": LaneSpec(
        "render.output-html",
        "render.output",
        "rendered_output",
        "rendered_html_access",
        "output-access",
        participants("access_rendered_html", REFKIT),
        "Access HTML from an already rendered refkit citation.",
    ),
    "render.output-tree": LaneSpec(
        "render.output-tree",
        "render.output",
        "rendered_output",
        "rendered_tree_access",
        "output-access",
        participants("access_rendered_tree", REFKIT),
        "Materialize the Python tree for an already rendered refkit citation.",
    ),
    "render.one-off-cite": LaneSpec(
        "render.one-off-cite",
        "render.one-off",
        "citation_rendering",
        "path_citation",
        "path-render",
        participants("render_path_citation", REFKIT, CITEPROC_PY),
        "Read BibTeX, resolve APA, and render one citation inside the operation.",
    ),
    "render.one-off-bibliography": LaneSpec(
        "render.one-off-bibliography",
        "render.one-off",
        "bibliography_rendering",
        "path_bibliography",
        "path-render",
        participants("render_path_bibliography", REFKIT, CITEPROC_PY),
        "Read BibTeX, resolve APA, and render a bibliography inside the operation.",
    ),
    "errors.missing-reference": LaneSpec(
        "errors.missing-reference",
        "errors",
        "error_contracts",
        "missing_reference",
        "error-resolution",
        participants("resolve_missing_reference", REFKIT, CITEPROC_PY),
        "Resolve one missing citation key through each renderer's public behavior.",
    ),
    "inspect.materialize": LaneSpec(
        "inspect.materialize",
        "inspect.entries",
        "entry_inspection",
        "entry_rows",
        "inspect",
        participants("materialize_entry_rows", REFKIT, BIBTEXPARSER_V2),
        "Materialize parsed entries into key and title rows after setup.",
    ),
    "inspect.keys": LaneSpec(
        "inspect.keys",
        "inspect.entries",
        "entry_inspection",
        "entry_keys",
        "inspect",
        participants("list_keys", REFKIT, BIBTEXPARSER_V2),
        "Enumerate citation keys after setup.",
    ),
    "inspect.lookup": LaneSpec(
        "inspect.lookup",
        "inspect.entries",
        "entry_inspection",
        "entry_lookup",
        "inspect",
        participants("lookup_entries", REFKIT, BIBTEXPARSER_V2),
        "Look up a fixed set of entries after setup.",
    ),
    "inspect.fields": LaneSpec(
        "inspect.fields",
        "inspect.entries",
        "entry_inspection",
        "field_projection",
        "inspect",
        participants("project_fields", REFKIT, BIBTEXPARSER_V2),
        "Project common scalar fields from all entries after setup.",
    ),
    "bulk.polars.materialize": LaneSpec(
        "bulk.polars.materialize",
        "bulk.polars",
        "bulk_tabular_processing",
        "tabular_entry_rows",
        "tabular",
        (
            participant(POLARS_REFKIT, "materialize_entry_rows_eager"),
            participant(POLARS_REFKIT, "materialize_entry_rows_lazy"),
        ),
        "Materialize entry rows through a Polars expression.",
    ),
    "bulk.polars.keys": LaneSpec(
        "bulk.polars.keys",
        "bulk.polars",
        "bulk_tabular_processing",
        "tabular_entry_keys",
        "tabular",
        (
            participant(POLARS_REFKIT, "list_keys_eager"),
            participant(POLARS_REFKIT, "list_keys_lazy"),
        ),
        "Enumerate citation keys through a Polars expression.",
    ),
    "bulk.polars.lookup": LaneSpec(
        "bulk.polars.lookup",
        "bulk.polars",
        "bulk_tabular_processing",
        "tabular_entry_lookup",
        "tabular",
        (
            participant(POLARS_REFKIT, "lookup_entries_eager"),
            participant(POLARS_REFKIT, "lookup_entries_lazy"),
        ),
        "Project and filter entries through a Polars expression.",
    ),
    "bulk.polars.fields": LaneSpec(
        "bulk.polars.fields",
        "bulk.polars",
        "bulk_tabular_processing",
        "tabular_field_projection",
        "tabular",
        (
            participant(POLARS_REFKIT, "project_fields_eager"),
            participant(POLARS_REFKIT, "project_fields_lazy"),
        ),
        "Project common scalar fields through a Polars expression.",
    ),
    "bulk.polars.citation": LaneSpec(
        "bulk.polars.citation",
        "bulk.polars",
        "bulk_tabular_processing",
        "tabular_citation",
        "tabular",
        (
            participant(POLARS_REFKIT, "render_citation_expression_eager"),
            participant(POLARS_REFKIT, "render_citation_expression_lazy"),
        ),
        "Render one citation through a Polars expression.",
    ),
    "bulk.polars.bibliography": LaneSpec(
        "bulk.polars.bibliography",
        "bulk.polars",
        "bulk_tabular_processing",
        "tabular_bibliography",
        "tabular",
        (
            participant(POLARS_REFKIT, "render_bibliography_expression_eager"),
            participant(POLARS_REFKIT, "render_bibliography_expression_lazy"),
        ),
        "Render a bibliography through a Polars expression.",
    ),
    "bulk.polars.repeated-citations": LaneSpec(
        "bulk.polars.repeated-citations",
        "bulk.polars",
        "bulk_tabular_processing",
        "tabular_citation_sequence",
        "tabular",
        (
            participant(POLARS_REFKIT, "render_citation_sequence_eager"),
            participant(POLARS_REFKIT, "render_citation_sequence_lazy"),
        ),
        "Render ordered citation batches through a Polars expression.",
    ),
}


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    if args.list:
        print_lane_list()
        return 0

    selected_lanes = select_lanes(args.lane, args.group)
    selected_inputs = select_inputs(args.input)
    result = run_suite(
        lane_names=selected_lanes,
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
    parser.add_argument("--list", action="store_true", help="List benchmark lanes and exit.")
    parser.add_argument("--lane", action="append", choices=sorted(LANES), help="Run one lane.")
    parser.add_argument(
        "--group", default="all", help="Run lanes from a group. Use all for every lane."
    )
    parser.add_argument(
        "--input", action="append", choices=[*SIZES, "all"], help="Input size to run."
    )
    parser.add_argument("--rounds", type=positive_int, default=5, help="Measured rounds per lane.")
    parser.add_argument(
        "--warmups", type=non_negative_int, default=2, help="Warmup rounds per lane."
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


def select_lanes(lane_names: list[str] | None, group: str) -> list[str]:
    if lane_names:
        return lane_names
    if group == "all":
        return list(LANES)
    selected = [lane.name for lane in LANES.values() if lane.group == group]
    if not selected:
        raise SystemExit(f"unknown benchmark lane group: {group}")
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
    lane_names: list[str],
    input_sizes: list[str],
    rounds: int,
    warmups: int,
    build_mode: str = "auto",
    package_adapters: list[PackageAdapter] | None = None,
) -> SuiteResult:
    suite_adapters = package_adapters if package_adapters is not None else adapters()
    adapters_by_name = {adapter.name: adapter for adapter in suite_adapters}
    metadata = machine_metadata(build_mode)
    rows: list[Row] = []

    with tempfile.TemporaryDirectory(prefix="refkit-benchmark-") as tmp:
        root = Path(tmp)
        for input_size in input_sizes:
            workload = materialize_workload(input_size, root)
            for lane_name in lane_names:
                lane = LANES[lane_name]
                for participant in lane.participants:
                    adapter = adapters_by_name.get(participant.package)
                    if adapter is None:
                        continue
                    rows.extend(
                        run_adapter_lane(
                            adapter=adapter,
                            lane=lane,
                            participant=participant,
                            workload=workload,
                            directory=root,
                            rounds=rounds,
                            warmups=warmups,
                            metadata=metadata,
                        )
                    )

    return {"metadata": metadata, "rows": rows}


def run_adapter_lane(
    *,
    adapter: PackageAdapter,
    lane: LaneSpec,
    participant: LaneParticipant,
    workload: Any,
    directory: Path,
    rounds: int,
    warmups: int,
    metadata: Metadata,
) -> list[Row]:
    base = base_row(adapter, lane, workload, metadata, rounds, warmups)
    setup_start = perf_counter()
    try:
        prepared = adapter.prepare(participant.adapter_operation, workload, directory)
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
                "execution_mode": "",
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
    operation_fields = prepared_row_fields(prepared, workload, setup_seconds, lane)

    try:
        for _ in range(warmups):
            try:
                outcome = prepared.operation()
                prepared.check(outcome)
            except Exception as exc:
                return [
                    {
                        **base,
                        **operation_fields,
                        "phase": lane.phase,
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
                elapsed = perf_counter() - start
                seconds = outcome.seconds if outcome.seconds is not None else elapsed
                prepared.check(outcome)
                rows.append(
                    {
                        **base,
                        **operation_fields,
                        "phase": lane.phase,
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
                        "phase": lane.phase,
                        "round": round_index,
                        "seconds": seconds,
                        "status": "failed",
                        "detail": repr(exc),
                    }
                )
                break
        return rows
    finally:
        try:
            prepared.cleanup()
        except Exception as exc:
            print(f"benchmark cleanup failed for {adapter.name}: {exc!r}", file=sys.stderr)


def base_row(
    adapter: PackageAdapter,
    lane: LaneSpec,
    workload: Any,
    metadata: Metadata,
    rounds: int,
    warmups: int,
) -> Row:
    adapter_version = adapter.version() or package_version(adapter.distribution)
    return {
        "lane": lane.name,
        "group": lane.group,
        "capability": lane.capability,
        "workflow": lane.workflow,
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
    lane: LaneSpec,
) -> Row:
    source_format = str(prepared.metadata.get("source_format", "unknown"))
    return {
        "operation_phase": lane.phase,
        "input_bytes": workload.source_byte_count(source_format),
        "input_sha256": workload.source_sha256(source_format),
        "source_format": source_format,
        "citation_count": int(prepared.metadata.get("citation_count", 0)),
        "execution_mode": str(prepared.metadata.get("execution_mode", "")),
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
            "polars-refkit": package_version("polars-refkit"),
            "citeproc-py": package_version("citeproc-py"),
            "bibtexparser": package_version("bibtexparser"),
            "bibtexparser-v2": package_version("bibtexparser"),
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


def print_lane_list() -> None:
    for lane in LANES.values():
        participants = ",".join(
            dict.fromkeys(participant.package for participant in lane.participants)
        )
        print(
            f"{lane.name}\t{lane.group}\t{lane.capability}\t"
            f"{lane.workflow}\t{participants}\t{lane.description}"
        )


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
