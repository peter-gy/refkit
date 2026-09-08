from __future__ import annotations

import argparse
import json
import random
import secrets
import sys
import tempfile
from datetime import datetime, timezone
from hashlib import sha256
from pathlib import Path
from typing import Any

import pyperf

from refkit_bench.cases import LANES, Case, select_cases
from refkit_bench.process import execute
from refkit_bench.provenance import environment, source_digest
from refkit_bench.report import compare, summarize, validate_measurement


def _positive(value: str) -> int:
    number = int(value)
    if number < 1:
        raise argparse.ArgumentTypeError("must be a positive integer")
    return number


def _seconds(value: str) -> float:
    import math

    number = float(value)
    if not math.isfinite(number) or number <= 0:
        raise argparse.ArgumentTypeError("must be a finite positive number of seconds")
    return number


def _selection(parser: argparse.ArgumentParser) -> None:
    parser.add_argument(
        "--lane",
        action="append",
        choices=[*LANES, "all"],
        default=None,
        help="Repeat to select capabilities. Default: parse.bibtex.",
    )
    parser.add_argument(
        "--dataset",
        action="append",
        help=(
            "Repeat names from list. Default: real, or all fixtures for format.spec. "
            "all includes scaling datasets."
        ),
    )
    parser.add_argument(
        "--package",
        action="append",
        help="Repeat participant names. Default: every participant in the selected lanes.",
    )


def parse_args(argv: list[str] | None = None) -> tuple[argparse.ArgumentParser, argparse.Namespace]:
    parser = argparse.ArgumentParser(description="Validate and measure bibliography capabilities.")
    commands = parser.add_subparsers(dest="command", required=True)
    listing = commands.add_parser("list", help="List selected cases and their operation contracts.")
    _selection(listing)
    listing.add_argument("--json", action="store_true")
    check = commands.add_parser(
        "check", help="Validate selected cases in isolated processes, collecting every failure."
    )
    _selection(check)
    check.add_argument("--output", type=Path, help="Write the correctness report as JSON.")
    check.add_argument("--timeout", type=_seconds, default=60)
    check.add_argument("--polars-threads", type=_positive, default=1)
    check.add_argument(
        "--affinity", help="CPU indexes or ranges, applied before participant setup."
    )
    run = commands.add_parser(
        "run", help="Validate every selected case, then collect calibrated pyperf measurements."
    )
    _selection(run)
    run.add_argument(
        "--output",
        type=Path,
        required=True,
        help="Create a new result directory containing manifest.json and timings.json.",
    )
    run.add_argument("--processes", type=_positive, default=10)
    run.add_argument("--values", type=_positive, default=5, help="Measured batches per worker.")
    run.add_argument(
        "--warmups",
        type=_positive,
        default=10,
        help="Warmup batches per worker, excluded from summaries.",
    )
    run.add_argument(
        "--min-time", type=_seconds, default=0.1, help="Target seconds per calibrated batch."
    )
    run.add_argument(
        "--timeout",
        type=_seconds,
        default=60,
        help=(
            "Worker budget in seconds. Windows bounds each case at "
            "(processes + 6) times this budget."
        ),
    )
    run.add_argument(
        "--seed", type=int, help="Case-order shuffle seed. Generated and recorded when omitted."
    )
    run.add_argument("--affinity", help="CPU indexes or ranges, applied before participant setup.")
    run.add_argument(
        "--polars-threads",
        type=_positive,
        default=1,
        help="Threads per Polars process, fixed before import. Default: 1.",
    )
    run.add_argument(
        "--smoke",
        action="store_true",
        help="One short worker to test the harness. Accepts debug builds and cannot be compared.",
    )
    for name in ("report", "compare"):
        command = commands.add_parser(name)
        if name == "compare":
            command.add_argument("baseline", type=Path)
        command.add_argument("result", type=Path)
        command.add_argument("--json", action="store_true")
    return parser, parser.parse_args(argv)


def _save(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    temporary.replace(path)


def check_cases(
    cases: list[Case], timeout: float, polars_threads: int = 1, affinity: str | None = None
) -> list[dict[str, Any]]:
    rows = []
    for case in cases:
        process = None
        try:
            command = [
                sys.executable,
                "-m",
                "refkit_bench.worker",
                "check",
                "--case",
                case.name,
                "--polars-threads",
                str(polars_threads),
            ]
            if affinity is not None:
                command.extend(["--affinity", affinity])
            process = execute(command, timeout)
            row = json.loads(process.stdout)
            if row.get("name") != case.name or row.get("status") not in {"ok", "failed"}:
                raise ValueError("worker returned an invalid correctness result")
            if process.returncode and row["status"] == "ok":
                raise ValueError("worker exited unsuccessfully after validation")
        except (ValueError, OSError, TimeoutError) as exc:
            detail = str(exc)
            if process is not None and process.stderr:
                detail += ": " + process.stderr[-2000:]
            row = {"name": case.name, "status": "failed", "detail": detail}
        rows.append(row)
        sys.stderr.write(
            f"{row['status']}: {case.name}"
            + (f": {row['detail']}" if row["status"] == "failed" else "")
            + "\n"
        )
    return rows


def run_cases(cases: list[Case], args: argparse.Namespace) -> int:
    args.output.mkdir(parents=True, exist_ok=False)
    seed = args.seed if args.seed is not None else secrets.randbits(32)
    measurement = {
        "processes": args.processes,
        "values": args.values,
        "warmups": args.warmups,
        "min_time": args.min_time,
        "affinity": args.affinity,
    }
    if args.smoke:
        measurement.update(processes=1, values=1, warmups=1, min_time=0.001)
    plan = list(cases)
    random.Random(seed).shuffle(plan)
    manifest: dict[str, Any] = {
        "schema": 1,
        "created_at": datetime.now(timezone.utc).isoformat(),
        "profile": "smoke" if args.smoke else "measurement",
        "status": "checking",
        "environment": environment(),
        "seed": seed,
        "execution_order": [case.name for case in plan],
        "measurement": measurement,
        "checks": [],
    }
    destination = args.output / "manifest.json"
    _save(destination, manifest)
    manifest["checks"] = check_cases(plan, args.timeout, args.polars_threads, args.affinity)
    if not args.smoke:
        for result in manifest["checks"]:
            package = result["name"].split("/")[-1]
            if (
                result["status"] == "ok"
                and package in {"refkit", "polars-eager", "polars-lazy"}
                and result["artifact"].get("build_mode") != "release"
            ):
                result.update(
                    status="failed",
                    detail=(
                        "timing requires an observed release build. Run "
                        "make refkit-develop-release polars-refkit-develop-release"
                    ),
                )
    if any(row["status"] != "ok" for row in manifest["checks"]):
        manifest["status"] = "failed"
        _save(destination, manifest)
        sys.stderr.write(f"Validation failed. Inspect {destination}.\n")
        return 1
    manifest["status"] = "measuring"
    _save(destination, manifest)
    benchmarks = []
    with tempfile.TemporaryDirectory(prefix="refkit-pyperf-") as temporary:
        for index, case in enumerate(plan):
            sys.stderr.write(f"measuring: {case.name}\n")
            output = Path(temporary) / f"{index}.json"
            command = [
                sys.executable,
                "-m",
                "refkit_bench.worker",
                "measure",
                "--copy-env",
                "--case",
                case.name,
                "--output",
                str(output),
                "--processes",
                str(measurement["processes"]),
                "--values",
                str(measurement["values"]),
                "--warmups",
                str(measurement["warmups"]),
                "--min-time",
                str(measurement["min_time"]),
            ]
            # pyperf's timed pipe reader uses select(), which accepts sockets on Windows.
            # The parent execute() call bounds and terminates the complete case process tree.
            if sys.platform != "win32":
                command.extend(["--timeout", str(args.timeout)])
            if args.smoke:
                command.append("--smoke")
            if args.affinity:
                command.extend(["--affinity", args.affinity])
            command.extend(["--polars-threads", str(args.polars_threads)])
            try:
                completed = execute(command, (int(measurement["processes"]) + 6) * args.timeout)
                if completed.returncode:
                    raise RuntimeError((completed.stderr or completed.stdout)[-4000:])
                suite = pyperf.BenchmarkSuite.load(str(output))
                expected = next(row for row in manifest["checks"] if row["name"] == case.name)
                for benchmark in suite:
                    validate_measurement(
                        benchmark, expected, str(manifest["environment"]["benchmark_sha256"])
                    )
                benchmarks.extend(suite.get_benchmarks())
                sys.stderr.write(completed.stdout)
            except (RuntimeError, ValueError, OSError, TimeoutError) as exc:
                manifest.update(status="failed", failed_case=case.name, detail=str(exc))
                _save(destination, manifest)
                return 1
            pyperf.BenchmarkSuite(benchmarks).dump(str(args.output / "timings.json"), replace=True)
    if source_digest() != manifest["environment"]["benchmark_sha256"]:
        manifest.update(status="failed", detail="benchmark sources changed during measurement")
        _save(destination, manifest)
        return 1
    manifest["timings_sha256"] = sha256((args.output / "timings.json").read_bytes()).hexdigest()
    manifest["status"] = "complete"
    _save(destination, manifest)
    _print_report(summarize(args.output), machine=False)
    return 0


def _print_report(value: dict[str, Any], *, machine: bool) -> None:
    if machine:
        sys.stdout.write(json.dumps(value, indent=2, ensure_ascii=False) + "\n")
        return
    if "ratio" in value:
        sys.stdout.write(value["ratio"] + "\n")
        sys.stdout.write("case\tbaseline\tcandidate\tratio\t95% interval\tassessment\n")
        for row in value["rows"]:
            interval = row["ratio_interval_95"]
            limits = "n/a" if interval is None else f"[{interval[0]:.3f}, {interval[1]:.3f}]"
            sys.stdout.write(
                f"{row['case']}\t{_duration(row['baseline_seconds'])}\t"
                f"{_duration(row['candidate_seconds'])}\t"
                f"{row['candidate_over_baseline']:.3f}x\t{limits}\t{row['assessment']}\n"
            )
    else:
        sys.stdout.write(f"{value['profile']}: {value['statistic']}\n")
        sys.stdout.write("case\tmedian\tworkers\tworker CV\n")
        for row in value["rows"]:
            variation = "n/a" if row["worker_cv"] is None else f"{row['worker_cv']:.1%}"
            sys.stdout.write(
                f"{row['case']}\t{_duration(row['seconds'])}\t{row['workers']}\t{variation}\n"
            )


def _duration(seconds: float) -> str:
    if seconds >= 1:
        return f"{seconds:.3g} s"
    if seconds >= 0.001:
        return f"{seconds * 1000:.3g} ms"
    return f"{seconds * 1_000_000:.3g} us"


def main(argv: list[str] | None = None) -> int:
    parser, args = parse_args(argv)
    try:
        if args.command == "report":
            _print_report(summarize(args.result), machine=args.json)
            return 0
        if args.command == "compare":
            _print_report(compare(args.baseline, args.result), machine=args.json)
            return 0
        cases = select_cases(args.lane or ["parse.bibtex"], args.dataset, args.package)
        if args.command == "list":
            rows = [
                {
                    "case": case.name,
                    "description": LANES[case.lane].description,
                    "unit": LANES[case.lane].unit,
                }
                for case in cases
            ]
            if args.json:
                sys.stdout.write(json.dumps(rows, indent=2) + "\n")
            else:
                for row in rows:
                    sys.stdout.write(f"{row['case']}\t{row['unit']}\t{row['description']}\n")
            return 0
        if args.command == "check":
            if args.output and args.output.exists():
                raise ValueError(f"{args.output} already exists. Choose a new report path.")
            result = {
                "schema": 1,
                "environment": environment(),
                "checks": check_cases(cases, args.timeout, args.polars_threads, args.affinity),
            }
            if args.output:
                _save(args.output, result)
            sys.stdout.write(json.dumps(result, indent=2, ensure_ascii=False) + "\n")
            return int(any(row["status"] != "ok" for row in result["checks"]))
        return run_cases(cases, args)
    except KeyboardInterrupt:
        if args.command == "run":
            destination = args.output / "manifest.json"
            if destination.is_file():
                manifest = json.loads(destination.read_text())
                manifest["status"] = "interrupted"
                _save(destination, manifest)
        sys.stderr.write("Benchmark interrupted.\n")
        return 130
    except (ValueError, OSError) as exc:
        parser.error(str(exc))
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
