from __future__ import annotations

import json
from collections.abc import Callable
from hashlib import sha256
from typing import TYPE_CHECKING, cast

from refkit_bench.model import MetadataValue, Prepared

if TYPE_CHECKING:
    from refkit.types import EncodeReport, Entry


STRESS_SIZES = {
    "raw.recover": {"unclosed-100": 100, "unclosed-1k": 1_000, "unclosed-5k": 5_000},
    "parse.recover": {"unknown-129": 129, "unknown-1k": 1_000, "unknown-5k": 5_000},
    "inspect.field": {"field-1kib": 1_024, "field-1mib": 1_048_576},
    "encode.csl": {"chunks-1k": 1_024, "chunks-8k": 8_192, "chunks-32k": 32_768},
}


def prepare_stress(lane: str, dataset: str) -> Prepared:
    factories: dict[str, Callable[[int], Prepared]] = {
        "raw.recover": _raw_recovery,
        "parse.recover": _report_recovery,
        "inspect.field": _field_metadata,
        "encode.csl": _chunk_encoding,
    }
    try:
        size = STRESS_SIZES[lane][dataset]
        factory = factories[lane]
    except KeyError as exc:
        raise ValueError(f"unknown stress case: {lane}/{dataset}") from exc
    return factory(size)


def _metadata(source: str, source_format: str, record_count: int) -> dict[str, MetadataValue]:
    encoded = source.encode()
    return {
        "input_sha256": sha256(encoded).hexdigest(),
        "input_bytes": len(encoded),
        "source_format": source_format,
        "record_count": record_count,
        "source_license": "Apache-2.0",
        "workload_family": "synthetic-boundary-stress",
        "provenance": "refkit-bench synthetic boundary inputs v1",
    }


def _raw_recovery(count: int) -> Prepared:
    import refkit as rk

    source = "".join(f"@misc{{broken{index},title={{\n" for index in range(count))
    source += "@misc{sentinel,title={Retained title}}"

    def check(result: object) -> None:
        document = cast(rk.BibDocument, result)
        if document.to_bibtex() != source:
            raise AssertionError("raw recovery changed source bytes")
        if document.entries.occurrence_keys() != ["sentinel"]:
            raise AssertionError("raw recovery lost the following valid entry")
        if document.entries["sentinel"].fields["title"].value != "Retained title":
            raise AssertionError("raw recovery changed the retained title")
        if len(document.failed_blocks) != count:
            raise AssertionError("raw recovery omitted malformed blocks")

    return Prepared(
        operation=lambda: rk.BibDocument.parse(source),
        check=check,
        metadata={
            **_metadata(source, "bibtex", count + 1),
            "malformed_block_count": count,
            "setup": "generated source in memory",
            "measured": "parse malformed source into a raw document",
            "validation": "exact source, retained entry and title, failed block count",
        },
    )


def _report_recovery(count: int) -> Prepared:
    import refkit as rk

    keys = [f"item{index}" for index in range(count)]
    titles = [f"Title {index}" for index in range(count)]
    source = "\n".join(
        f"@misc{{{key},title={{{title}}},custom=missing{index}}}"
        for index, (key, title) in enumerate(zip(keys, titles, strict=True))
    )

    def check(result: object) -> None:
        library = cast(rk.Library, result)
        if library.keys() != keys:
            raise AssertionError("report recovery changed the ordered entry set")
        if library.project(["title"]) != [{"title": title} for title in titles]:
            raise AssertionError("report recovery changed retained titles")
        diagnostics = library.diagnostics
        if len(diagnostics) != count or any(
            item["code"] != "unknown_abbreviation" or item["action"] != "literalized"
            for item in diagnostics
        ):
            raise AssertionError("report recovery omitted literalization diagnostics")

    return Prepared(
        operation=lambda: rk.Library.parse_bibtex(source, recovery="report"),
        check=check,
        metadata={
            **_metadata(source, "bibtex", count),
            "unknown_atom_count": count,
            "setup": "generated source in memory",
            "measured": "recover and normalize every entry with diagnostics",
            "validation": "ordered keys, titles, and literalization diagnostic count",
        },
    )


def _field_metadata(size: int) -> Prepared:
    import refkit as rk

    prefix = "@misc{item,abstract={"
    source = prefix + "x" * size + "}}"
    field = rk.BibDocument.parse(source).entries["item"].fields["abstract"]
    expected = ("abstract", (len(prefix), len(prefix) + size))

    def check(result: object) -> None:
        if result != expected:
            raise AssertionError("field metadata changed its name or value span")

    return Prepared(
        operation=lambda: (field.name, field.span),
        check=check,
        metadata={
            **_metadata(source, "bibtex", 1),
            "field_value_bytes": size,
            "setup": "parse raw document and retain one field handle",
            "measured": "read field name and value span into Python values",
            "validation": "exact field name and independently calculated span",
        },
    )


def _chunk_encoding(count: int) -> Prepared:
    import refkit as rk

    records: list[Entry] = [
        {
            "key": "item",
            "entry_type": "Book",
            "title": {"chunks": [{"kind": "normal", "text": "x"} for _ in range(count)]},
        }
    ]
    source = json.dumps(records, sort_keys=True, separators=(",", ":"))
    library = rk.Library.from_records(records)
    expected = [{"id": "item", "type": "book", "title": "x" * count}]

    def check(result: object) -> None:
        report = cast("EncodeReport", result)
        if report["format"] != "csl-json" or json.loads(report["text"]) != expected:
            raise AssertionError("CSL encoding changed the complete record")
        if report["issues"]:
            raise AssertionError("CSL encoding reported unexpected conversion issues")

    return Prepared(
        operation=lambda: rk.encode(library, format="csl-json"),
        check=check,
        metadata={
            **_metadata(source, "refkit-records", 1),
            "text_chunk_count": count,
            "setup": "construct a library from adjacent normal title chunks",
            "measured": "encode the prepared library and materialize its CSL JSON report",
            "validation": "complete decoded CSL record and empty conversion issues",
        },
    )
