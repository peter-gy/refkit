from __future__ import annotations

import json
from dataclasses import replace
from typing import cast

import pytest

from refkit_bench import runner
from refkit_bench.cases import Case
from refkit_bench.stress import STRESS_SIZES


@pytest.mark.parametrize(
    ("lane", "dataset"),
    [(lane, dataset) for lane, sizes in STRESS_SIZES.items() for dataset in sizes],
)
def test_stress_operations_satisfy_their_public_contract(lane: str, dataset: str) -> None:
    prepared = Case(lane, dataset, "refkit").prepare()
    prepared.check(prepared.operation())
    prepared.check(prepared.operation())


def test_stress_cli_lists_available_scales_by_default(capsys: pytest.CaptureFixture[str]) -> None:
    assert runner.main(["list", "--lane", "raw.recover", "--json"]) == 0
    cases = {row["case"] for row in json.loads(capsys.readouterr().out)}
    assert "raw.recover/unclosed-5k/refkit" in cases
    assert all(case.startswith("raw.recover/") for case in cases)


def test_raw_recovery_check_rejects_source_loss() -> None:
    import refkit as rk

    prepared = Case("raw.recover", "unclosed-100", "refkit").prepare()
    document = cast(rk.BibDocument, prepared.operation())
    with pytest.raises(AssertionError, match="source bytes"):
        prepared.check(rk.BibDocument.parse(document.to_bibtex() + "\n"))


def test_report_recovery_check_rejects_entry_loss_and_missing_diagnostics() -> None:
    import refkit as rk

    prepared = Case("parse.recover", "unknown-129", "refkit").prepare()
    library = cast(rk.Library, prepared.operation())
    records = library.to_records()
    with pytest.raises(AssertionError, match="ordered entry set"):
        prepared.check(rk.Library.from_records(records[:-1]))
    with pytest.raises(AssertionError, match="literalization diagnostics"):
        prepared.check(rk.Library.from_records(records))
    records[0]["title"] = {"chunks": [{"kind": "normal", "text": "Changed"}]}
    with pytest.raises(AssertionError, match="retained titles"):
        prepared.check(rk.Library.from_records(records))


def test_field_metadata_check_rejects_wrong_name_or_span() -> None:
    prepared = Case("inspect.field", "field-1mib", "refkit").prepare()
    name, span = cast(tuple[str, tuple[int, int]], prepared.operation())
    for wrong in [("title", span), (name, (span[0], span[1] - 1))]:
        with pytest.raises(AssertionError, match="name or value span"):
            prepared.check(wrong)


def test_chunk_encoding_check_rejects_content_loss_and_conversion_issues() -> None:
    from refkit.types import EncodeReport

    prepared = Case("encode.csl", "chunks-1k", "refkit").prepare()
    report = cast(EncodeReport, prepared.operation())
    with pytest.raises(AssertionError, match="complete record"):
        prepared.check({**report, "text": "[]"})
    with pytest.raises(AssertionError, match="conversion issues"):
        prepared.check({**report, "issues": [{"code": "loss"}]})


def test_stress_case_hashes_identify_the_actual_input() -> None:
    case = Case("inspect.field", "field-1kib", "refkit")
    first = case.prepare().metadata
    repeated = case.prepare().metadata
    larger = replace(case, dataset="field-1mib").prepare().metadata
    assert first == repeated
    assert first["input_sha256"] != larger["input_sha256"]
    first_bytes, larger_bytes = first["input_bytes"], larger["input_bytes"]
    assert isinstance(first_bytes, int) and isinstance(larger_bytes, int)
    assert larger_bytes - first_bytes == 1_048_576 - 1_024
