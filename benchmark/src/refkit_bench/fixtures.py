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
LARGEST_SIZE = "large"
WORKLOAD_NAMES = (*SIZES, "arxiv")
WORKLOAD_FAMILY = "synthetic_scale"
WORKLOAD_SOURCE_LICENSE = "Apache-2.0"
ARXIV_WORKLOAD_FAMILY = "arxiv_wild_subset"
ARXIV_WORKLOAD_SOURCE_LICENSE = "mixed-arxiv-source-licenses"

JOURNAL = "Journal of Citation Benchmarks"
REPO_ROOT = Path(__file__).resolve().parents[3]
ARXIV_SUBSET_PATH = REPO_ROOT / "data" / "arxiv-wild" / "references-subset.bib"
PACKAGED_ARXIV_SUBSET_PATH = (
    Path(__file__).resolve().parent / "data" / "arxiv-wild" / "references-subset.bib"
)


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
    bibliography_terms: tuple[str, ...] = ()

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
        count = SIZES[size]
    except KeyError as exc:
        raise ValueError(f"unknown workload size: {size}") from exc

    return largest_records()[:count]


def largest_records() -> tuple[Record, ...]:
    return tuple(_record(index) for index in range(1, SIZES[LARGEST_SIZE] + 1))


def materialize_workload(size: str, directory: Path) -> Workload:
    if size == "arxiv":
        return materialize_arxiv_workload(directory)

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


def materialize_arxiv_workload(directory: Path) -> Workload:
    bibtex = arxiv_bibtex()
    raw_bibtex = bibtex
    dirty_bibtex = bibtex
    records = arxiv_records()
    duplicate_bibtex = duplicate_bibtex_for_records(records)
    bibtex_path = directory / "arxiv.bib"
    raw_bibtex_path = directory / "arxiv-raw.bib"
    dirty_bibtex_path = directory / "arxiv-dirty.bib"
    duplicate_bibtex_path = directory / "arxiv-duplicates.bib"
    bibtex_path.write_text(bibtex, encoding="utf-8")
    raw_bibtex_path.write_text(raw_bibtex, encoding="utf-8")
    dirty_bibtex_path.write_text(dirty_bibtex, encoding="utf-8")
    duplicate_bibtex_path.write_text(duplicate_bibtex, encoding="utf-8")
    return Workload(
        size="arxiv",
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
        family_name=ARXIV_WORKLOAD_FAMILY,
        source_license_name=ARXIV_WORKLOAD_SOURCE_LICENSE,
        raw_preservation_terms=("Real BibTeX subset",),
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
    if entries:
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
    duplicate = ""
    if records:
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


def audited_tiny_bibtex() -> str:
    return (Path(__file__).parent / "data" / "tiny.bib").read_text(encoding="utf-8")


def arxiv_bibtex() -> str:
    return arxiv_subset_path().read_text(encoding="utf-8")


def arxiv_subset_path() -> Path:
    if ARXIV_SUBSET_PATH.exists():
        return ARXIV_SUBSET_PATH
    if PACKAGED_ARXIV_SUBSET_PATH.exists():
        return PACKAGED_ARXIV_SUBSET_PATH
    raise FileNotFoundError("arxiv workload fixture is missing")


def arxiv_records() -> tuple[Record, ...]:
    return (
        _arxiv_record(
            "ijcai2019p684",
            "Chen",
            "Huimin",
            "Sentiment-Controllable Chinese Poetry Generation",
            2019,
            "10.24963/ijcai.2019/684",
            volume=None,
            pages="4925-4931",
            container=(
                "Proceedings of the Twenty-Eighth International Joint Conference on "
                "Artificial Intelligence, IJCAI-19"
            ),
            item_type="paper-conference",
            authors=(
                ("Chen", "Huimin"),
                ("Yi", "Xiaoyuan"),
                ("Sun", "Maosong"),
                ("Li", "Wenhao"),
                ("Yang", "Cheng"),
                ("Guo", "Zhipeng"),
            ),
            citation_text="(Chen et al., 2019)",
        ),
        _arxiv_record(
            "10.1145/3325887",
            "Liu",
            "Dayiheng",
            "Ancient–Modern Chinese Translation with a New Large Training Dataset",
            2019,
            "10.1145/3325887",
            volume="19",
            container="ACM Trans. Asian Low-Resour. Lang. Inf. Process.",
            authors=(
                ("Liu", "Dayiheng"),
                ("Yang", "Kexin"),
                ("Qu", "Qian"),
                ("Lv", "Jiancheng"),
            ),
            citation_text="(Liu et al., 2019)",
        ),
        _arxiv_record(
            "Kimi_K2.5",
            "Team",
            "Kimi",
            "Kimi K2.5: Visual Agentic Intelligence",
            2026,
            "10.48550/ARXIV.2602.02276",
            volume="abs/2602.02276",
            raw_title="Kimi {K2.5:} Visual Agentic Intelligence",
            authors=(("Team", "Kimi"),),
            citation_text="(Team, 2026)",
        ),
        _arxiv_record(
            "DeepSeek-V3.2",
            "DeepSeek-AI",
            "",
            "DeepSeek-V3.2: Pushing the Frontier of Open Large Language Models",
            2025,
            "10.48550/ARXIV.2512.02556",
            volume="abs/2512.02556",
            authors=(("DeepSeek-AI", ""),),
            citation_text="(DeepSeek-AI, 2025)",
        ),
        _arxiv_record(
            "DeepResearchGym",
            "Coelho",
            "João",
            (
                "DeepResearchGym: A Free, Transparent, and Reproducible Evaluation "
                "Sandbox for Deep Research"
            ),
            2025,
            "10.48550/ARXIV.2505.19253",
            volume="abs/2505.19253",
            raw_title=(
                "DeepResearchGym: {A} Free, Transparent, and Reproducible "
                "Evaluation Sandbox for Deep Research"
            ),
            authors=(("Coelho", "João"), ("Ning", "Jingjie"), ("Chang", "Michael")),
            citation_text="(Coelho et al., 2025)",
        ),
        _arxiv_record(
            "BioAgent_Bench",
            "Fa",
            "Dionizije",
            "BioAgent Bench: An AI Agent Evaluation Suite for Bioinformatics",
            2026,
            "10.48550/ARXIV.2601.21800",
            volume="abs/2601.21800",
            raw_title="BioAgent Bench: An {AI} Agent Evaluation Suite for Bioinformatics",
            authors=(("Fa", "Dionizije"), ("Culjak", "Marko"), ("Pando", "Bruno")),
            citation_text="(Fa et al., 2026)",
        ),
        _arxiv_record(
            "AutoEnv",
            "Zhang",
            "Jiayi",
            "AutoEnv: Automated Environments for Measuring Cross-Environment Agent Learning",
            2025,
            "10.48550/ARXIV.2511.19304",
            volume="abs/2511.19304",
            authors=(("Zhang", "Jiayi"), ("Peng", "Yiran"), ("Kong", "Fanqi")),
            citation_text="(Zhang et al., 2025)",
        ),
        _arxiv_record(
            "AgentSynth",
            "Xie",
            "Jingxu",
            "AgentSynth: Scalable Task Generation for Generalist Computer-Use Agents",
            2025,
            "10.48550/ARXIV.2506.14205",
            volume="abs/2506.14205",
            authors=(("Xie", "Jingxu"), ("Xu", "Dylan"), ("Zhao", "Xuandong")),
            citation_text="(Xie et al., 2025)",
        ),
        _arxiv_record(
            "AutoForge",
            "Cai",
            "Shihao",
            "AutoForge: Automated Environment Synthesis for Agentic Reinforcement Learning",
            2025,
            "10.48550/ARXIV.2512.22857",
            volume="abs/2512.22857",
            authors=(("Cai", "Shihao"), ("Fang", "Runnan"), ("Wu", "Jialong")),
            citation_text="(Cai et al., 2025)",
        ),
        _arxiv_record(
            "EnvScaler",
            "Song",
            "Xiaoshuai",
            (
                "EnvScaler: Scaling Tool-Interactive Environments for LLM Agent "
                "via Programmatic Synthesis"
            ),
            2026,
            "10.48550/ARXIV.2601.05808",
            volume="abs/2601.05808",
            raw_title=(
                "EnvScaler: Scaling Tool-Interactive Environments for {LLM} "
                "Agent via Programmatic Synthesis"
            ),
            authors=(("Song", "Xiaoshuai"), ("Chang", "Haofei"), ("Feng", "Xiaodong")),
            citation_text="(Song et al., 2026)",
        ),
        _arxiv_record(
            "TaskCraft",
            "Shi",
            "Dingfeng",
            "TaskCraft: Automated Generation of Agentic Tasks",
            2025,
            "10.48550/ARXIV.2506.10055",
            volume="abs/2506.10055",
            authors=(("Shi", "Dingfeng"), ("Cao", "Jingyi"), ("Chen", "Qianben")),
            citation_text="(Shi et al., 2025)",
        ),
        _arxiv_record(
            "Agent2World",
            "Hu",
            "Mengkang",
            (
                "Agent2World: Learning to Generate Symbolic World Models via "
                "Adaptive Multi-Agent Feedback"
            ),
            2025,
            "10.48550/ARXIV.2512.22336",
            volume="abs/2512.22336",
            authors=(("Hu", "Mengkang"), ("Xia", "Bowei"), ("Wu", "Yuran")),
            citation_text="(Hu et al., 2025)",
        ),
    )


def _arxiv_record(
    key: str,
    family: str,
    given: str,
    title: str,
    year: int,
    doi: str,
    *,
    volume: str | None,
    raw_title: str = "",
    pages: str = "",
    container: str = "Corr",
    item_type: str = "article-journal",
    authors: tuple[tuple[str, str], ...],
    citation_text: str,
) -> Record:
    return Record(
        key=key,
        family=family,
        given=given,
        title=title,
        year=year,
        volume=volume,
        page_start=None,
        page_end=None,
        doi=doi,
        raw_title=raw_title,
        container=container,
        pages=pages,
        item_type=item_type,
        authors=authors,
        citation_text=citation_text,
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
    lines = [
        f"@article{{{record.key},\n"
        f"  author = {{{record.family}, {record.given}}},\n"
        f"  title = {{{record.title}}},\n"
        f"  journal = {{{record.container}}},\n"
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
