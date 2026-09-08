from __future__ import annotations

from collections.abc import Callable
from hashlib import sha256
from typing import Any, cast

from refkit_bench.fixtures import Record, Workload
from refkit_bench.model import Prepared
from refkit_bench.rendering import LOCALE, STYLE_PATH, expected_bibliography

EDITED_TITLE = "Edited Benchmark Title"
RAW_HEADER = (
    "% benchmark raw comment\n"
    "@string{benchjournal = {Journal of Citation Benchmarks}}\n"
    '@preamble{"Benchmark preamble"}\n'
)


def prepare_parse(lane: str, workload: Workload, package: str) -> Prepared:
    """Prepare in-memory parsing, inspection, or a fresh raw edit workflow."""
    if lane not in {
        "parse.bibtex",
        "inspect.keys",
        "inspect.lookup",
        "inspect.project",
        "raw.edit",
    }:
        raise ValueError(f"unknown parsing lane: {lane}")
    if package not in {"refkit", "bibtexparser", "pybtex"}:
        raise ValueError(f"unknown parsing participant: {package}")
    if lane == "raw.edit":
        return _raw_edit(workload, package)
    parser = _parser(package)
    expected = [_record_fields(record) for record in workload.records]
    if lane == "parse.bibtex":
        if package == "refkit":
            import refkit as rk

            style = rk.Style.from_xml(STYLE_PATH.read_text(encoding="utf-8"))
            locale = rk.Locale.load(LOCALE)
            bibliography = expected_bibliography(workload.records)

            def check(result: object) -> None:
                library = cast(rk.Library, result)
                if library.diagnostics:
                    raise AssertionError(f"unexpected parser diagnostics: {library.diagnostics!r}")
                rows = [_refkit_fields(entry) for entry in library.values()]
                partial = [
                    {key: value for key, value in row.items() if key not in {"author", "pages"}}
                    for row in expected
                ]
                _equal(rows, partial)
                # Rendering exposes authors and page ranges through the public Library contract.
                _equal(
                    rk.Document(library, style, locale=locale)
                    .full_bibliography()
                    .text.splitlines(),
                    bibliography,
                )
        else:

            def check(result: object) -> None:
                _equal(_parsed_fields(result, package), expected)

        return Prepared(
            operation=lambda: parser(workload.bibtex),
            check=check,
            metadata={
                "source_format": "bibtex",
                "setup": "input text in memory",
                "measured": "parse into native queryable model",
                "validation": (
                    "public entry fields and complete authored-style bibliography outside timing"
                    if package == "refkit"
                    else "complete native fields and ordered author names outside timing"
                ),
            },
        )

    library = parser(workload.bibtex)
    keys = workload.keys
    indices = tuple(dict.fromkeys((0, len(keys) // 2, len(keys) - 1)))
    selected = [keys[index] for index in indices]
    fields = ("key", "title") if lane == "inspect.lookup" else ("key", "title", "doi", "volume")
    if lane == "inspect.keys":
        expected_output: object = keys
    else:
        records = (
            [workload.records[index] for index in indices]
            if lane == "inspect.lookup"
            else workload.records
        )
        expected_output = [
            {
                "key": record.key,
                "title": record.title,
                **(
                    {
                        "doi": record.doi,
                        "volume": str(record.volume) if record.volume is not None else None,
                    }
                    if lane == "inspect.project"
                    else {}
                ),
            }
            for record in records
        ]

    def operation() -> object:
        if package == "refkit":
            if lane == "inspect.keys":
                return library.keys()
            if lane == "inspect.lookup":
                return [
                    {"key": entry.key, "title": entry.title} for entry in library.get_many(selected)
                ]
            return library.project(fields)
        if package == "bibtexparser":
            if lane == "inspect.keys":
                return list(library.entries_dict)
            entries = library.entries_dict
            chosen = selected if lane == "inspect.lookup" else entries
            return [_project_bibtexparser(entries[key], fields) for key in chosen]
        if lane == "inspect.keys":
            return list(library.entries)
        chosen = selected if lane == "inspect.lookup" else library.entries
        return [_project_pybtex(key, library.entries[key], fields) for key in chosen]

    return Prepared(
        operation=operation,
        check=lambda result: _equal(result, expected_output),
        metadata={
            "source_format": "bibtex",
            "setup": "parse into native queryable model",
            "measured": "native lookup and Python result materialization",
            "selected_count": len(selected) if lane == "inspect.lookup" else len(keys),
        },
    )


def _parser(package: str) -> Callable[[str], Any]:
    if package == "refkit":
        import refkit

        return refkit.Library.parse_bibtex
    if package == "bibtexparser":
        import bibtexparser

        return bibtexparser.parse_string
    from pybtex.database import parse_string

    return lambda source: parse_string(source, "bibtex")


def _record_fields(record: Record) -> dict[str, object]:
    return {
        "key": record.key,
        "type": "inproceedings" if record.item_type == "paper-conference" else "article",
        "title": record.title,
        "author": " and ".join(
            f"{family}, {given}" if given else family
            for family, given in record.authors or ((record.family, record.given),)
        ),
        "year": str(record.year),
        "container": record.container,
        "volume": str(record.volume) if record.volume is not None else None,
        "pages": record.page_range,
        "doi": record.doi,
    }


def _refkit_fields(entry: Any) -> dict[str, object]:
    parents = entry.parents
    if entry.entry_type != "Article" or len(parents) != 1:
        raise AssertionError(
            f"unexpected entry structure for {entry.key}: {entry.entry_type}, {parents!r}"
        )
    parent = parents[0]
    kind = {"Proceedings": "inproceedings", "Periodical": "article"}.get(parent.entry_type)
    return {
        "key": entry.key,
        "type": kind,
        "title": entry.title,
        "year": entry.date,
        "container": parent.title,
        "volume": entry.volume,
        "doi": entry.doi,
    }


def _parsed_fields(library: Any, package: str) -> list[dict[str, object]]:
    if package == "bibtexparser":
        if library.failed_blocks:
            raise AssertionError(f"unexpected failed parser blocks: {library.failed_blocks!r}")
        return [
            _fields(entry.key, entry.entry_type, {field.key: field.value for field in entry.fields})
            for entry in library.entries
        ]
    return [
        _fields(
            key,
            entry.type,
            {
                **entry.fields,
                "author": " and ".join(str(person) for person in entry.persons.get("author", [])),
            },
        )
        for key, entry in library.entries.items()
    ]


def _fields(key: str, kind: str, fields: dict[str, str]) -> dict[str, object]:
    return {
        "key": key,
        "type": kind,
        "title": _title(fields.get("title", "")),
        "author": fields.get("author", ""),
        "year": fields.get("year", ""),
        "container": fields.get("booktitle" if kind == "inproceedings" else "journal", ""),
        "volume": fields.get("volume"),
        "pages": fields.get("pages", "").replace("--", "-").replace("–", "-"),
        "doi": fields.get("doi"),
    }


def _title(value: str) -> str:
    return value.replace("{", "").replace("}", "")


def _project_bibtexparser(entry: Any, fields: tuple[str, ...]) -> dict[str, object]:
    values = entry.fields_dict
    row: dict[str, object] = {"key": entry.key, "title": _title(values["title"].value)}
    if "doi" in fields:
        row["doi"] = values["doi"].value if "doi" in values else None
        row["volume"] = values["volume"].value if "volume" in values else None
    return row


def _project_pybtex(key: str, entry: Any, fields: tuple[str, ...]) -> dict[str, object]:
    row: dict[str, object] = {"key": key, "title": _title(entry.fields["title"])}
    if "doi" in fields:
        row["doi"] = entry.fields.get("doi")
        row["volume"] = entry.fields.get("volume")
    return row


def _raw_edit(workload: Workload, package: str) -> Prepared:
    if package == "pybtex":
        raise ValueError("raw.edit participants are refkit and bibtexparser")
    import bibtexparser

    source = RAW_HEADER + workload.bibtex
    key = workload.keys[0]
    if package == "refkit":
        import refkit

        def operation() -> object:
            document = refkit.BibDocument.parse(source)
            document.entries[key].fields["title"].value = EDITED_TITLE
            return document.to_bibtex()
    else:

        def operation() -> object:
            document = bibtexparser.parse_string(source)
            document.entries_dict[key].fields_dict["title"].value = EDITED_TITLE
            return bibtexparser.write_string(document)

    expected = [_raw_fields(record) for record in workload.records]
    expected[0]["fields"]["title"] = EDITED_TITLE

    def check(result: object) -> None:
        if not isinstance(result, str):
            raise AssertionError("raw edit must return BibTeX text")
        document = bibtexparser.parse_string(result)
        if document.failed_blocks:
            raise AssertionError(
                f"edited BibTeX contains failed blocks: {document.failed_blocks!r}"
            )
        actual = [
            {
                "key": entry.key,
                "type": entry.entry_type,
                "fields": {field.key: field.value for field in entry.fields},
            }
            for entry in document.entries
        ]
        _equal(actual, expected)
        _equal(
            [comment.comment.strip() for comment in document.comments], ["% benchmark raw comment"]
        )
        _equal(
            [(string.key, string.value) for string in document.strings],
            [("benchjournal", "Journal of Citation Benchmarks")],
        )
        _equal([preamble.value for preamble in document.preambles], ['"Benchmark preamble"'])

    return Prepared(
        operation=operation,
        check=check,
        metadata={
            "source_format": "bibtex",
            "input_sha256": sha256(source.encode()).hexdigest(),
            "input_bytes": len(source.encode()),
            "setup": "input text with comment, string, and preamble in memory",
            "measured": "fresh parse, first title edit, serialization to memory",
        },
    )


def _raw_fields(record: Record) -> dict[str, Any]:
    fields = _record_fields(record)
    kind = str(fields["type"])
    return {
        "key": record.key,
        "type": kind,
        "fields": {
            "author": fields["author"],
            "title": record.raw_title or record.title,
            "booktitle" if kind == "inproceedings" else "journal": record.container,
            "year": str(record.year),
            **({"volume": str(record.volume)} if record.volume is not None else {}),
            **({"pages": record.page_range} if record.page_range else {}),
            **({"doi": record.doi} if record.doi else {}),
        },
    }


def _equal(actual: object, expected: object) -> None:
    if actual != expected:
        raise AssertionError(f"output mismatch: expected {expected!r}, got {actual!r}")
