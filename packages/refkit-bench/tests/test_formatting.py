from __future__ import annotations

from dataclasses import replace

import pytest

from refkit_bench.fixtures import bibtex_for_records, csl_json_for_records, load_workload
from refkit_bench.formatting import FormatCase, formatting_cases, prepare_format


@pytest.mark.parametrize("case", formatting_cases(), ids=lambda case: case.name)
def test_refkit_matches_curated_formatter_contract(case: FormatCase):
    prepared = prepare_format(case, "refkit")
    try:
        prepared.check(prepared.operation())
    finally:
        prepared.close()


def test_node_batch_returns_public_output_and_worker_clock():
    prepared = prepare_format(formatting_cases()[0], "bibtex-tidy")
    try:
        assert prepared.measure is not None
        elapsed, output = prepared.measure(3)
        assert elapsed > 0
        prepared.check(output)
        assert prepared.metadata["clock"] == "process.hrtime.bigint"
        assert prepared.metadata["package_version"] == "1.14.0"
        assert prepared.operation() == output
    finally:
        prepared.close()


@pytest.mark.parametrize("package", ["refkit", "bibtex-tidy"])
def test_formatter_gate_rejects_damaged_expected_output(package: str):
    case = replace(formatting_cases()[0], expected="@article{damaged}\n")
    prepared = prepare_format(case, package)
    try:
        with pytest.raises(AssertionError, match="exact formatter output"):
            prepared.check(prepared.operation())
    finally:
        prepared.close()


def test_node_protocol_error_closes_the_owned_process():
    from refkit_bench.formatting import _NodeWorker

    worker = _NodeWorker()
    try:
        worker.request({"input": "@article{a,title={A}}", "options": {}})
        with pytest.raises(RuntimeError, match="positive safe-integer loop count"):
            worker.request({"loops": 0})
        assert worker.process.poll() is not None
        assert not worker.reader.is_alive()
    finally:
        worker.close()


def test_node_response_timeout_closes_the_owned_process(monkeypatch: pytest.MonkeyPatch):
    from queue import Empty

    from refkit_bench.formatting import _NodeWorker

    worker = _NodeWorker()

    def timed_out(*args, **kwargs):
        raise Empty

    try:
        monkeypatch.setattr(worker.responses, "get", timed_out)
        with pytest.raises(RuntimeError, match="protocol failed"):
            worker.request({"loops": 1})
        assert worker.process.poll() is not None
        assert not worker.reader.is_alive()
    finally:
        worker.close()


@pytest.mark.parametrize("message", ["{broken\n", "null\n"])
def test_malformed_node_response_closes_the_owned_process(message: str):
    from refkit_bench.formatting import _NodeWorker

    worker = _NodeWorker()
    try:
        worker.responses.put(message)
        with pytest.raises(RuntimeError, match="protocol failed"):
            worker.request({"loops": 1})
        assert worker.process.poll() is not None
        assert not worker.reader.is_alive()
    finally:
        worker.close()


@pytest.mark.parametrize("field", ["input", "options"])
def test_formatter_fixture_integrity_is_verified_before_loading(tmp_path, monkeypatch, field: str):
    import json

    from refkit_bench import formatting

    document = json.loads((formatting._DATA / "cases.json").read_text())
    document["cases"][0][field] = "@article{changed}" if field == "input" else {"space": 8}
    (tmp_path / "cases.json").write_text(json.dumps(document))
    monkeypatch.setattr(formatting, "_DATA", tmp_path)
    with pytest.raises(ValueError, match=f"invalid {field} hash"):
        formatting_cases()


def test_generated_key_suffixes_continue_from_z_to_aa():
    import refkit
    from refkit_bench.formatting import key_case

    workload = load_workload("tiny")
    records = tuple(replace(workload.records[0], key=f"entry{index}") for index in range(27))
    workload = replace(
        workload,
        size="collision-boundary",
        records=records,
        bibtex=bibtex_for_records(records),
        csl_json=csl_json_for_records(records),
    )
    prepared = prepare_format(key_case(workload), "refkit")
    result = prepared.operation()
    prepared.check(result)
    assert isinstance(result, refkit.TidyResult)
    keys = refkit.BibDocument.parse(result.bibtex).entries.unique_keys()
    assert len(keys) == 27
    assert keys[0] == "2001a"
    assert keys[25:] == ["2001z", "2001aa"]
