from __future__ import annotations

import json
from dataclasses import asdict, dataclass
from hashlib import sha256

from refkit_bench.fixtures import WORKLOAD_NAMES, load_workload
from refkit_bench.formatting import formatting_cases, key_case, layout_case, prepare_format
from refkit_bench.model import Prepared


@dataclass(frozen=True)
class Lane:
    description: str
    packages: tuple[str, ...]
    unit: str


LANES = {
    "parse.bibtex": Lane(
        "Parse an in-memory bibliography into a queryable model.",
        ("refkit", "bibtexparser", "pybtex"),
        "bibliography",
    ),
    "inspect.keys": Lane(
        "List keys from a prepared library.", ("refkit", "bibtexparser", "pybtex"), "library"
    ),
    "inspect.lookup": Lane(
        "Fetch first, middle, and last entries as key/title rows.",
        ("refkit", "bibtexparser", "pybtex"),
        "three-key request",
    ),
    "inspect.project": Lane(
        "Project key, title, DOI, and volume from every entry.",
        ("refkit", "bibtexparser", "pybtex"),
        "library",
    ),
    "raw.edit": Lane(
        "Parse, change one title, and serialize a BibTeX document.",
        ("refkit", "bibtexparser"),
        "document edit",
    ),
    "render.citation": Lane(
        "Render one citation from prepared data and a shared style.",
        ("refkit", "citeproc-py"),
        "citation request",
    ),
    "render.bibliography": Lane(
        "Render the complete ordered bibliography from prepared data.",
        ("refkit", "citeproc-py"),
        "bibliography",
    ),
    "render.document": Lane(
        "Render ordered citations and their bibliography together.",
        ("refkit", "citeproc-py"),
        "citation document",
    ),
    "format.layout": Lane(
        "Apply the shared explicit layout profile to a bibliography.",
        ("refkit", "bibtex-tidy"),
        "bibliography",
    ),
    "format.keys": Lane(
        "Generate year-based keys and resolve collisions while formatting.",
        ("refkit", "bibtex-tidy"),
        "bibliography",
    ),
    "format.spec": Lane(
        "Check one exact upstream transformation specification.",
        ("refkit", "bibtex-tidy"),
        "specification input",
    ),
    "batch.parse": Lane(
        "Parse independent one-entry BibTeX rows with a Polars expression.",
        ("polars-eager", "polars-lazy"),
        "dataframe",
    ),
    "batch.cite": Lane(
        "Render an APA citation for each independent bibliography row.",
        ("polars-eager", "polars-lazy"),
        "dataframe",
    ),
}


def fingerprint(value: object) -> str:
    return sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
    ).hexdigest()


@dataclass(frozen=True)
class Case:
    lane: str
    dataset: str
    package: str

    @property
    def name(self) -> str:
        return f"{self.lane}/{self.dataset}/{self.package}"

    def prepare(self) -> Prepared:
        if self.lane == "format.spec":
            fixture = next(case for case in formatting_cases() if case.name == self.dataset)
            prepared = prepare_format(fixture, self.package)
        else:
            workload = load_workload(self.dataset)
            if self.lane in {"format.layout", "format.keys"}:
                factory = key_case if self.lane == "format.keys" else layout_case
                prepared = prepare_format(factory(workload), self.package)
            elif self.lane.startswith("render."):
                from refkit_bench.rendering import prepare_render

                prepared = prepare_render(self.lane, workload, self.package)
            elif self.lane.startswith("batch."):
                from refkit_bench.tabular import prepare_batch

                prepared = prepare_batch(self.lane, workload, self.package)
            else:
                from refkit_bench.parsing import prepare_parse

                prepared = prepare_parse(self.lane, workload, self.package)
            source = workload.bibtex
            if prepared.metadata.get("source_format") == "csl_json":
                source = json.dumps(
                    workload.csl_json, sort_keys=True, separators=(",", ":"), ensure_ascii=False
                )
            prepared.metadata = {
                "input_sha256": sha256(source.encode()).hexdigest(),
                "input_bytes": len(source.encode()),
                "record_count": workload.record_count,
                "records_sha256": fingerprint([asdict(record) for record in workload.records]),
                "source_license": workload.source_license_name,
                "workload_family": workload.family_name,
                "provenance": workload.provenance,
                **prepared.metadata,
            }
        prepared.metadata.update(
            {
                "lane": self.lane,
                "dataset": self.dataset,
                "participant": self.package,
                "unit_of_work": LANES[self.lane].unit,
            }
        )
        return prepared


def select_cases(
    lanes: list[str], datasets: list[str] | None = None, packages: list[str] | None = None
) -> list[Case]:
    selected_lanes = list(LANES) if "all" in lanes else list(dict.fromkeys(lanes))
    unknown = set(selected_lanes) - LANES.keys()
    if unknown:
        raise ValueError(f"unknown lanes: {', '.join(sorted(unknown))}")
    specs = tuple(case.name for case in formatting_cases())
    selected_datasets = list(dict.fromkeys(datasets or ["real"]))
    known_datasets = {*WORKLOAD_NAMES, *specs, "all"}
    if invalid := set(selected_datasets) - known_datasets:
        raise ValueError(f"unknown datasets: {', '.join(sorted(invalid))}")
    cases = []
    for lane in selected_lanes:
        available = specs if lane == "format.spec" else WORKLOAD_NAMES
        names = (
            available
            if "all" in selected_datasets or (lane == "format.spec" and datasets is None)
            else [name for name in selected_datasets if name in available]
        )
        participants = [
            name for name in LANES[lane].packages if packages is None or name in packages
        ]
        cases.extend(Case(lane, name, package) for name in names for package in participants)
    if packages and (unmatched := set(packages) - {case.package for case in cases}):
        raise ValueError(f"participants have no selected cases: {', '.join(sorted(unmatched))}")
    if not cases:
        raise ValueError("the selected lanes, datasets, and participants have no cases in common")
    return cases


def case_by_name(name: str) -> Case:
    parts = name.split("/")
    if len(parts) != 3:
        raise ValueError(f"invalid case name: {name}")
    lane, dataset, package = parts
    cases = select_cases([lane], [dataset], [package])
    return cases[0]
