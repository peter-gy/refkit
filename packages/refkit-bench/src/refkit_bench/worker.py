from __future__ import annotations

import argparse
import gc
import json
import os
import sys
from time import perf_counter
from typing import Any

import pyperf

from refkit_bench import affinity
from refkit_bench.cases import case_by_name, fingerprint
from refkit_bench.provenance import artifact, source_digest

_ARTIFACT_FIELDS = {
    "artifact_sha256",
    "artifact_path",
    "package_version",
    "node_version",
    "v8_version",
    "build_mode",
    "artifact_source_url",
    "artifact_source_revision",
    "runtime",
    "python_flags",
}


def inspect_case(name: str, cpus: tuple[int, ...] | None = None) -> dict[str, Any]:
    case = case_by_name(name)
    prepared = None
    started = perf_counter()
    result: dict[str, Any] = {"name": name, "status": "failed"}
    try:
        prepared = case.prepare()
        prepared.metadata["effective_affinity"] = affinity.describe(cpus)
        affinity.verify(cpus)
        result["setup_seconds"] = perf_counter() - started
        metadata = {**artifact(case.package), **prepared.metadata}
        result["case_sha256"] = fingerprint({"case": case.name, "metadata": prepared.metadata})
        result["artifact"] = {
            key: value for key, value in metadata.items() if key in _ARTIFACT_FIELDS
        }
        result["contract"] = {
            key: json.loads(str(value)) if key == "options_json" else value
            for key, value in metadata.items()
            if key not in _ARTIFACT_FIELDS
        }
        if "options_json" in result["contract"]:
            result["contract"]["options"] = result["contract"].pop("options_json")
        prepared.check(prepared.operation())
        affinity.verify(cpus)
        result["status"] = "ok"
    except Exception as exc:
        result["detail"] = f"{type(exc).__name__}: {exc}"[:2000]
    finally:
        if prepared is not None:
            try:
                prepared.close()
            except Exception as exc:
                result.update(status="failed", detail=f"cleanup failed: {exc}")
    return result


def measure() -> None:
    def arguments(command: list[str], args: argparse.Namespace) -> None:
        command.extend(["--case", args.case])
        command.extend(["--polars-threads", str(args.polars_threads)])
        if args.smoke:
            command.append("--smoke")

    runner = pyperf.Runner(
        program_args=("-m", "refkit_bench.worker", "measure"), add_cmdline_args=arguments
    )
    runner.argparser.add_argument("--case", required=True)
    runner.argparser.add_argument("--smoke", action="store_true")
    runner.argparser.add_argument("--polars-threads", type=int, default=1)
    args = runner.parse_args()
    cpus = affinity.configure(args.affinity)
    if cpus is not None:
        args.affinity = affinity.describe(cpus)
    os.environ["POLARS_MAX_THREADS"] = str(args.polars_threads)
    case = case_by_name(args.case)
    prepared = case.prepare()
    try:
        prepared.metadata["effective_affinity"] = affinity.describe(cpus)
        affinity.verify(cpus)
        metadata = {**artifact(case.package), **prepared.metadata}
        if (
            case.package in {"refkit", "polars-eager", "polars-lazy"}
            and metadata.get("build_mode") != "release"
            and not args.smoke
        ):
            raise RuntimeError(
                "timing requires an observed release build. Run "
                "make refkit-develop-release polars-refkit-develop-release"
            )
        if not args.smoke and (sys.gettrace() is not None or sys.getprofile() is not None):
            raise RuntimeError("measurement requires an uninstrumented interpreter")
        gc.enable()
        prepared.check(prepared.operation())
        metadata.update(
            benchmark_sha256=source_digest(),
            case_sha256=fingerprint({"case": case.name, "metadata": prepared.metadata}),
            gc_enabled=1,
            profile="smoke" if args.smoke else "measurement",
        )

        def batch(loops: int) -> float:
            seconds = prepared.time_batch(loops)
            affinity.verify(cpus)
            return seconds

        runner.bench_time_func(case.name, batch, metadata=metadata)
    finally:
        prepared.close()


def main() -> None:
    mode = sys.argv.pop(1)
    if mode == "check":
        parser = argparse.ArgumentParser()
        parser.add_argument("--case", required=True)
        parser.add_argument("--polars-threads", type=int, default=1)
        parser.add_argument("--affinity")
        args = parser.parse_args()
        cpus = affinity.configure(args.affinity)
        os.environ["POLARS_MAX_THREADS"] = str(args.polars_threads)
        result = inspect_case(args.case, cpus)
        sys.stdout.write(json.dumps(result, ensure_ascii=False) + "\n")
        raise SystemExit(0 if result["status"] == "ok" else 1)
    if mode != "measure":
        raise SystemExit(f"unknown worker mode: {mode}")
    measure()


if __name__ == "__main__":
    main()
