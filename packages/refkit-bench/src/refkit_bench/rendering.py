from __future__ import annotations

from collections.abc import Callable
from hashlib import sha256
from io import BytesIO
from pathlib import Path

from refkit_bench.fixtures import Record, Workload
from refkit_bench.model import Prepared

STYLE_PATH = Path(__file__).parent / "data" / "styles" / "author-date.csl"
LOCALE = "en-US"


def prepare_render(lane: str, workload: Workload, package: str) -> Prepared:
    """Prepare exact author-date rendering checks and fresh-processor operations."""
    if lane not in {"render.citation", "render.bibliography", "render.document"}:
        raise ValueError(f"unknown rendering lane: {lane}")
    if not workload.records:
        raise ValueError("rendering requires at least one record")
    style_xml = STYLE_PATH.read_bytes()
    if package == "refkit":
        operation = _refkit(lane, workload, style_xml)
    elif package == "citeproc-py":
        operation = _citeproc(lane, workload, style_xml)
    else:
        raise ValueError(f"unknown rendering participant: {package}")

    citations = [_citation(record) for record in workload.records]
    bibliography = expected_bibliography(workload.records)
    expected: object
    if lane == "render.citation":
        expected = citations[0]
    elif lane == "render.bibliography":
        expected = bibliography
    else:
        expected = {"citations": citations, "bibliography": bibliography}

    def check(actual: object) -> None:
        if actual != expected:
            raise AssertionError(f"{lane} output mismatch: expected {expected!r}, got {actual!r}")

    return Prepared(
        operation=operation,
        check=check,
        metadata={
            "style_sha256": sha256(style_xml).hexdigest(),
            "style_name": "RefKit benchmark author-date",
            "style_license": "Apache-2.0",
            "locale": LOCALE,
            "source_format": "bibtex" if package == "refkit" else "csl_json",
            "setup": "parse source and style, resolve locale",
            "measured": "fresh processor, requests, rendering, plain-text materialization",
            "citation_count": 1 if lane == "render.citation" else len(citations),
            "citation_api_bibliography": int(package == "refkit"),
        },
    )


def expected_bibliography(records: tuple[Record, ...]) -> list[str]:
    """Return complete bibliography text for the authored benchmark style."""
    return [
        _bibliography(record)
        for record in sorted(records, key=lambda record: record.title.casefold())
    ]


def _citation(record: Record) -> str:
    families = " / ".join(
        family for family, _ in record.authors or ((record.family, record.given),)
    )
    return f"({families}, {record.year})"


def _bibliography(record: Record) -> str:
    authors = " / ".join(
        f"{family}, {given}" if given else family
        for family, given in record.authors or ((record.family, record.given),)
    )
    fields = [authors, str(record.year), record.title, record.container]
    if record.volume is not None:
        fields.append(str(record.volume))
    if record.page_range:
        fields.append(record.page_range.replace("-", "–"))
    if record.doi:
        fields.append(record.doi)
    return " | ".join(field for field in fields if field)


def _refkit(lane: str, workload: Workload, style_xml: bytes) -> Callable[[], object]:
    import refkit as rk

    library = rk.Library.parse_bibtex(workload.bibtex)
    style = rk.Style.from_xml(style_xml.decode("utf-8"))
    locale = rk.Locale.load(LOCALE)
    keys = workload.keys

    def operation() -> object:
        document = rk.Document(library, style, locale=locale)
        if lane == "render.bibliography":
            return document.full_bibliography().text.splitlines()
        selected = keys[:1] if lane == "render.citation" else keys
        requests = [rk.Citation(key, key) for key in selected]
        rendered = document.render(requests)
        if lane == "render.citation":
            return rendered[selected[0]].text
        return {
            "citations": [rendered[key].text for key in rendered.citation_order],
            "bibliography": rendered.bibliography.text.splitlines(),
        }

    return operation


def _citeproc(lane: str, workload: Workload, style_xml: bytes) -> Callable[[], object]:
    from citeproc import (
        Citation,
        CitationItem,
        CitationStylesBibliography,
        CitationStylesStyle,
        formatter,
    )
    from citeproc.source.json import CiteProcJSON

    source = CiteProcJSON(workload.csl_json)
    style = CitationStylesStyle(BytesIO(style_xml), locale=LOCALE, validate=True)
    keys = workload.keys

    def operation() -> object:
        processor = CitationStylesBibliography(style, source, formatter.plain)
        selected = keys[:1] if lane == "render.citation" else keys
        requests = [Citation([CitationItem(key)]) for key in selected]
        for request in requests:
            processor.register(request)
        if lane == "render.citation":
            return str(processor.cite(requests[0], _missing))
        citations = (
            [str(processor.cite(request, _missing)) for request in requests]
            if lane == "render.document"
            else []
        )
        processor.sort()
        bibliography = [str(row) for row in processor.bibliography()]
        if lane == "render.bibliography":
            return bibliography
        return {"citations": citations, "bibliography": bibliography}

    return operation


def _missing(item: object) -> None:
    raise AssertionError(f"citation source lookup failed: {item!r}")
