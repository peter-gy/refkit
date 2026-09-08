from __future__ import annotations

import json
from dataclasses import dataclass
from hashlib import sha256
from pathlib import Path

SIZES = {"tiny": 3, "medium": 48, "large": 192}
SCALING_SIZES = {"1k": 1_000, "5k": 5_000, "10k": 10_000}
WORKLOAD_NAMES = (*SIZES, "real", *SCALING_SIZES)
JOURNAL = "Journal of Citation Benchmarks"
REAL_RECORDS_PATH = Path(__file__).with_name("data") / "real-bibliography" / "records.json"


@dataclass(frozen=True)
class Record:
    key: str
    family: str
    given: str
    title: str
    year: int
    volume: int | str | None
    page_start: int | None
    page_end: int | None
    doi: str | None
    raw_title: str = ""
    container: str = JOURNAL
    pages: str = ""
    item_type: str = "article-journal"
    authors: tuple[tuple[str, str], ...] = ()
    citation_text: str = ""

    @property
    def page_range(self) -> str:
        if self.pages:
            return self.pages
        if self.page_start is None or self.page_end is None:
            return ""
        return f"{self.page_start}-{self.page_end}"


@dataclass(frozen=True)
class Workload:
    size: str
    records: tuple[Record, ...]
    bibtex: str
    csl_json: list[dict[str, object]]
    source_license_name: str
    family_name: str
    provenance: str

    @property
    def keys(self) -> list[str]:
        return [record.key for record in self.records]

    @property
    def record_count(self) -> int:
        return len(self.records)


def load_workload(name: str) -> Workload:
    records = real_records() if name == "real" else records_for_size(name)
    return Workload(
        size=name,
        records=records,
        bibtex=bibtex_for_records(records),
        csl_json=csl_json_for_records(records),
        source_license_name="CC0-1.0" if name == "real" else "Apache-2.0",
        family_name="curated-metadata" if name == "real" else "fixed-shape-scaling",
        provenance=(
            "records.json:" + sha256(REAL_RECORDS_PATH.read_bytes()).hexdigest()
            if name == "real"
            else "refkit-bench synthetic records v1"
        ),
    )


def records_for_size(size: str) -> tuple[Record, ...]:
    try:
        count = {**SIZES, **SCALING_SIZES}[size]
    except KeyError as exc:
        raise ValueError(f"unknown workload size: {size}") from exc

    return tuple(_record(index) for index in range(1, count + 1))


def bibtex_for_records(records: tuple[Record, ...]) -> str:
    return "\n\n".join(_bibtex_entry(record) for record in records) + "\n"


def csl_json_for_records(records: tuple[Record, ...]) -> list[dict[str, object]]:
    items: list[dict[str, object]] = []
    for record in records:
        authors = record.authors or ((record.family, record.given),)
        item: dict[str, object] = {
            "id": record.key,
            "type": record.item_type,
            "title": record.title,
            "author": [{"family": family, "given": given} for family, given in authors],
            "issued": {"date-parts": [[record.year]]},
        }
        if record.container:
            item["container-title"] = record.container
        if record.volume is not None:
            item["volume"] = str(record.volume)
        if record.page_range:
            item["page"] = record.page_range
        if record.doi:
            item["DOI"] = record.doi
        items.append(item)
    return items


def real_records() -> tuple[Record, ...]:
    path = REAL_RECORDS_PATH
    records = json.loads(path.read_text(encoding="utf-8"))
    return tuple(
        Record(
            family=record["authors"][0][0],
            given=record["authors"][0][1],
            page_start=None,
            page_end=None,
            authors=tuple(tuple(author) for author in record["authors"]),
            **{
                key: value
                for key, value in record.items()
                if key not in {"source_url", "source_status", "authors"}
            },
        )
        for record in records
    )


def _record(index: int) -> Record:
    return Record(
        key=f"item{index:04d}",
        family=f"Family{index:04d}",
        given=f"Given{index:04d}",
        title=f"Reference Work {index:04d}",
        year=2000 + (index % 25),
        volume=1 + (index % 12),
        page_start=index * 3,
        page_end=index * 3 + 8,
        doi=f"10.5555/refkit.bench.{index:04d}",
    )


def _bibtex_entry(record: Record) -> str:
    authors = " and ".join(
        f"{family}, {given}" if given else family
        for family, given in (record.authors or ((record.family, record.given),))
    )
    entry_type = "inproceedings" if record.item_type == "paper-conference" else "article"
    container_field = "booktitle" if record.item_type == "paper-conference" else "journal"
    lines = [
        f"@{entry_type}{{{record.key},\n"
        f"  author = {{{authors}}},\n"
        f"  title = {{{record.raw_title or record.title}}},\n"
        f"  {container_field} = {{{record.container}}},\n"
        f"  year = {{{record.year}}},\n"
    ]
    if record.volume is not None:
        lines.append(f"  volume = {{{record.volume}}},\n")
    if record.page_range:
        lines.append(f"  pages = {{{record.page_range}}},\n")
    if record.doi:
        lines.append(f"  doi = {{{record.doi}}}\n")
    lines.append("}")
    return "".join(lines)
