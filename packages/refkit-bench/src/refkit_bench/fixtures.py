from __future__ import annotations

import json
from dataclasses import dataclass
from hashlib import sha256
from pathlib import Path

SIZES: dict[str, int] = {
    "tiny": 3,
    "medium": 48,
    "large": 192,
}
SCALING_SIZES = {"1k": 1_000, "5k": 5_000, "10k": 10_000}
WORKLOAD_NAMES = (*SIZES, "real")
WORKLOAD_FAMILY = "synthetic_scale"
WORKLOAD_SOURCE_LICENSE = "Apache-2.0"
REAL_WORKLOAD_FAMILY = "real_bibliography_subset"
REAL_WORKLOAD_SOURCE_LICENSE = "CC0-1.0"

FAILED_BIBTEX = "@article{unfinished, title = {Missing closing braces"
JOURNAL = "Journal of Citation Benchmarks"
REAL_RECORDS_PATH = Path(__file__).resolve().parent / "data" / "real-bibliography" / "records.json"


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
    raw_bibtex: str
    dirty_bibtex: str
    duplicate_bibtex: str
    csl_json: list[dict[str, object]]
    bibtex_path: Path
    raw_bibtex_path: Path
    dirty_bibtex_path: Path
    duplicate_bibtex_path: Path
    family_name: str = WORKLOAD_FAMILY
    source_license_name: str = WORKLOAD_SOURCE_LICENSE
    raw_preservation_terms: tuple[str, ...] = ()
    duplicate_entry_key: str = ""
    duplicate_field_key: str = ""
    duplicate_field_name: str = "title"

    @property
    def keys(self) -> list[str]:
        return [record.key for record in self.records]

    @property
    def family(self) -> str:
        return self.family_name

    @property
    def record_count(self) -> int:
        return len(self.records)

    def source_text(self, source_format: str) -> str:
        if source_format == "failed_bibtex":
            return FAILED_BIBTEX
        if source_format == "bibtex":
            return self.bibtex
        if source_format == "raw_bibtex":
            return self.raw_bibtex
        if source_format == "dirty_bibtex":
            return self.dirty_bibtex
        if source_format == "duplicate_bibtex":
            return self.duplicate_bibtex
        if source_format == "csl_json":
            return json.dumps(self.csl_json, sort_keys=True, separators=(",", ":"))
        return ""

    def source_name(self, source_format: str) -> str:
        if not self.source_text(source_format):
            return ""
        return f"{self.family}:{self.size}:{source_format}"

    def source_path(self, source_format: str) -> str:
        if source_format == "bibtex":
            return str(self.bibtex_path)
        if source_format == "raw_bibtex":
            return str(self.raw_bibtex_path)
        if source_format == "dirty_bibtex":
            return str(self.dirty_bibtex_path)
        if source_format == "duplicate_bibtex":
            return str(self.duplicate_bibtex_path)
        return ""

    def source_license(self, source_format: str) -> str:
        if source_format == "failed_bibtex":
            return WORKLOAD_SOURCE_LICENSE
        if not self.source_text(source_format):
            return ""
        return self.source_license_name

    def source_byte_count(self, source_format: str) -> int:
        return len(self.source_text(source_format).encode("utf-8"))

    def source_sha256(self, source_format: str) -> str:
        text = self.source_text(source_format)
        if not text:
            return ""
        return sha256(text.encode("utf-8")).hexdigest()


def records_for_size(size: str) -> tuple[Record, ...]:
    try:
        count = {**SIZES, **SCALING_SIZES}[size]
    except KeyError as exc:
        raise ValueError(f"unknown workload size: {size}") from exc

    return tuple(_record(index) for index in range(1, count + 1))


def materialize_workload(size: str, directory: Path) -> Workload:
    if size == "real":
        return materialize_real_workload(directory)

    records = records_for_size(size)
    bibtex = bibtex_for_records(records)
    raw_bibtex = raw_bibtex_for_records(records)
    dirty_bibtex = dirty_bibtex_for_records(records)
    duplicate_bibtex = duplicate_bibtex_for_records(records)
    bibtex_path = directory / f"{size}.bib"
    raw_bibtex_path = directory / f"{size}-raw.bib"
    dirty_bibtex_path = directory / f"{size}-dirty.bib"
    duplicate_bibtex_path = directory / f"{size}-duplicates.bib"
    bibtex_path.write_text(bibtex, encoding="utf-8")
    raw_bibtex_path.write_text(raw_bibtex, encoding="utf-8")
    dirty_bibtex_path.write_text(dirty_bibtex, encoding="utf-8")
    duplicate_bibtex_path.write_text(duplicate_bibtex, encoding="utf-8")
    return Workload(
        size=size,
        records=records,
        bibtex=bibtex,
        raw_bibtex=raw_bibtex,
        dirty_bibtex=dirty_bibtex,
        duplicate_bibtex=duplicate_bibtex,
        csl_json=csl_json_for_records(records),
        bibtex_path=bibtex_path,
        raw_bibtex_path=raw_bibtex_path,
        dirty_bibtex_path=dirty_bibtex_path,
        duplicate_bibtex_path=duplicate_bibtex_path,
        raw_preservation_terms=(
            "benchmark fixture with raw BibTeX blocks",
            "benchjournal",
            "Reference benchmark fixture",
        ),
        duplicate_entry_key=records[0].key,
        duplicate_field_key=records[1].key,
    )


def materialize_real_workload(directory: Path) -> Workload:
    records = real_records()
    bibtex = bibtex_for_records(records)
    raw_bibtex = "% Curated bibliography metadata\n" + bibtex
    dirty_bibtex = bibtex
    duplicate_bibtex = duplicate_bibtex_for_records(records)
    bibtex_path = directory / "real.bib"
    raw_bibtex_path = directory / "real-raw.bib"
    dirty_bibtex_path = directory / "real-dirty.bib"
    duplicate_bibtex_path = directory / "real-duplicates.bib"
    bibtex_path.write_text(bibtex, encoding="utf-8")
    raw_bibtex_path.write_text(raw_bibtex, encoding="utf-8")
    dirty_bibtex_path.write_text(dirty_bibtex, encoding="utf-8")
    duplicate_bibtex_path.write_text(duplicate_bibtex, encoding="utf-8")
    return Workload(
        size="real",
        records=records,
        bibtex=bibtex,
        raw_bibtex=raw_bibtex,
        dirty_bibtex=dirty_bibtex,
        duplicate_bibtex=duplicate_bibtex,
        csl_json=csl_json_for_records(records),
        bibtex_path=bibtex_path,
        raw_bibtex_path=raw_bibtex_path,
        dirty_bibtex_path=dirty_bibtex_path,
        duplicate_bibtex_path=duplicate_bibtex_path,
        family_name=REAL_WORKLOAD_FAMILY,
        source_license_name=REAL_WORKLOAD_SOURCE_LICENSE,
        raw_preservation_terms=("Curated bibliography metadata",),
        duplicate_entry_key=records[0].key,
        duplicate_field_key=records[1].key,
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


def dirty_bibtex_for_records(records: tuple[Record, ...]) -> str:
    entries = [_bibtex_entry(record) for record in records]
    first = records[0]
    entries[0] = (
        f"@article{{{first.key},\n"
        f"  author = {{{first.family}, {first.given}}},\n"
        f"  title = {{{first.title}}},\n"
        "  journal = JMLR # { Extra},\n"
        f"  year = {{{first.year}}},\n"
        "  month = {16},\n"
        f"  volume = {{{first.volume}}},\n"
        f"  pages = {{{first.page_start}-{first.page_end}}},\n"
        f"  doi = {{{first.doi}}}\n"
        "}"
    )
    duplicate = (
        "\n\n"
        f"@article{{{records[0].key},\n"
        "  title = {Duplicate benchmark record},\n"
        "  year = {2024}\n"
        "}\n"
    )
    return "\n\n".join(entries) + "\n\n@broken{missing,\n  title = {No close}\n" + duplicate


def duplicate_bibtex_for_records(records: tuple[Record, ...]) -> str:
    if len(records) < 3:
        raise ValueError("duplicate benchmark source requires at least three records")
    entry_duplicate = records[0]
    field_duplicate = records[1]
    steady = records[2]
    return (
        f"@article{{{entry_duplicate.key},\n"
        f"  title = {{{entry_duplicate.title}}},\n"
        f"  year = {{{entry_duplicate.year}}}\n"
        "}\n\n"
        f"@article{{{entry_duplicate.key},\n"
        "  title = {Duplicate benchmark entry},\n"
        f"  year = {{{entry_duplicate.year + 1}}}\n"
        "}\n\n"
        f"@article{{{field_duplicate.key},\n"
        f"  title = {{{field_duplicate.title}}},\n"
        "  title = {Duplicate benchmark field},\n"
        f"  year = {{{field_duplicate.year}}}\n"
        "}\n\n"
        f"@article{{{steady.key},\n"
        f"  title = {{{steady.title}}},\n"
        f"  year = {{{steady.year}}}\n"
        "}\n"
    )


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
