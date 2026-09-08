from __future__ import annotations

import json
from hashlib import sha256

from refkit_bench.fixtures import Workload, bibtex_for_records
from refkit_bench.model import Prepared
from refkit_bench.rendering import LOCALE


def prepare_batch(lane: str, workload: Workload, package: str) -> Prepared:
    import polars as pl

    import polars_refkit as prk

    if lane not in {"batch.parse", "batch.cite"} or package not in {"polars-eager", "polars-lazy"}:
        raise ValueError(f"unknown Polars case: {lane}/{package}")
    sources = [bibtex_for_records((record,)) for record in workload.records]
    frame = pl.DataFrame({"bib": sources, "key": workload.keys})
    lazy = package == "polars-lazy"
    if lane == "batch.parse":
        expected: object = [
            {
                "result": [
                    {
                        "key": record.key,
                        "title": record.title,
                        "doi": record.doi,
                        "volume": None if record.volume is None else str(record.volume),
                    }
                ]
            }
            for record in workload.records
        ]
    else:
        expected = [
            {"result": record.citation_text or f"({record.family}, {record.year})"}
            for record in workload.records
        ]

    def operation() -> object:
        if lane == "batch.parse":
            expression = prk.entries("bib", fields=["key", "title", "doi", "volume"])
        else:
            expression = prk.cite("bib", "key", style="apa", locale=LOCALE)
        if lazy:
            result = frame.lazy().select(expression.alias("result")).collect()
        else:
            result = frame.select(expression.alias("result"))
        return result.to_dicts()

    def check(actual: object) -> None:
        if actual != expected:
            raise AssertionError(
                f"{lane}: expected complete ordered output for {len(sources)} bibliography rows"
            )

    source = json.dumps(sources, ensure_ascii=False, separators=(",", ":"))
    metadata: dict[str, str | int | float] = {
        "input_sha256": sha256(source.encode()).hexdigest(),
        "input_bytes": sum(len(item.encode()) for item in sources),
        "source_format": "bibtex rows",
        "row_count": len(sources),
        "entries_per_row": 1,
        "execution_mode": "lazy" if lazy else "eager",
        "polars_version": pl.__version__,
        "polars_threads": pl.thread_pool_size(),
        "setup": "construct input dataframe",
        "measured": "expression construction, execution, Python row materialization",
    }
    if lane == "batch.cite":
        metadata.update(
            style_name="apa", style_source="bundled in measured Polars plugin", locale=LOCALE
        )
    return Prepared(operation=operation, check=check, metadata=metadata)
