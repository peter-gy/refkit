from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

SIZES: dict[str, int] = {
    "tiny": 3,
    "medium": 48,
    "large": 192,
}
LARGEST_SIZE = "large"

JOURNAL = "Journal of Citation Benchmarks"


@dataclass(frozen=True)
class Record:
    key: str
    family: str
    given: str
    title: str
    year: int
    volume: int
    page_start: int
    page_end: int
    doi: str


@dataclass(frozen=True)
class Workload:
    size: str
    records: tuple[Record, ...]
    bibtex: str
    raw_bibtex: str
    csl_json: list[dict[str, object]]
    bibtex_path: Path
    raw_bibtex_path: Path

    @property
    def keys(self) -> list[str]:
        return [record.key for record in self.records]


def records_for_size(size: str) -> tuple[Record, ...]:
    try:
        count = SIZES[size]
    except KeyError as exc:
        raise ValueError(f"unknown workload size: {size}") from exc

    return largest_records()[:count]


def largest_records() -> tuple[Record, ...]:
    return tuple(_record(index) for index in range(1, SIZES[LARGEST_SIZE] + 1))


def materialize_workload(size: str, directory: Path) -> Workload:
    records = records_for_size(size)
    bibtex = bibtex_for_records(records)
    raw_bibtex = raw_bibtex_for_records(records)
    bibtex_path = directory / f"{size}.bib"
    raw_bibtex_path = directory / f"{size}-raw.bib"
    bibtex_path.write_text(bibtex, encoding="utf-8")
    raw_bibtex_path.write_text(raw_bibtex, encoding="utf-8")
    return Workload(
        size=size,
        records=records,
        bibtex=bibtex,
        raw_bibtex=raw_bibtex,
        csl_json=csl_json_for_records(records),
        bibtex_path=bibtex_path,
        raw_bibtex_path=raw_bibtex_path,
    )


def bibtex_for_records(records: tuple[Record, ...]) -> str:
    return "\n\n".join(_bibtex_entry(record) for record in records) + "\n"


def raw_bibtex_for_records(records: tuple[Record, ...]) -> str:
    body = bibtex_for_records(records)
    return (
        "% benchmark fixture with raw BibTeX blocks\n"
        "@string{benchjournal = {Journal of Citation Benchmarks}}\n"
        "@preamble{Reference benchmark fixture}\n\n"
        f"{body}"
    )


def csl_json_for_records(records: tuple[Record, ...]) -> list[dict[str, object]]:
    return [
        {
            "id": record.key,
            "type": "article-journal",
            "title": record.title,
            "author": [{"family": record.family, "given": record.given}],
            "issued": {"date-parts": [[record.year]]},
            "container-title": JOURNAL,
            "volume": str(record.volume),
            "page": f"{record.page_start}-{record.page_end}",
            "DOI": record.doi,
        }
        for record in records
    ]


def audited_tiny_bibtex() -> str:
    return (Path(__file__).parent / "data" / "tiny.bib").read_text(encoding="utf-8")


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
    return (
        f"@article{{{record.key},\n"
        f"  author = {{{record.family}, {record.given}}},\n"
        f"  title = {{{record.title}}},\n"
        f"  journal = {{{JOURNAL}}},\n"
        f"  year = {{{record.year}}},\n"
        f"  volume = {{{record.volume}}},\n"
        f"  pages = {{{record.page_start}-{record.page_end}}},\n"
        f"  doi = {{{record.doi}}}\n"
        "}"
    )
