from __future__ import annotations

import pytest

from refkit_bench.fixtures import load_workload
from refkit_bench.tabular import prepare_batch


@pytest.mark.parametrize("package", ["polars-eager", "polars-lazy"])
@pytest.mark.parametrize("lane", ["batch.parse", "batch.cite"])
def test_independent_bibliography_rows_execute_and_materialize(lane, package):
    prepared = prepare_batch(lane, load_workload("tiny"), package)
    result = prepared.operation()
    prepared.check(result)
    assert isinstance(result, list)
    assert len(result) == 3
    assert prepared.metadata["row_count"] == 3
    assert prepared.metadata["entries_per_row"] == 1
    if lane == "batch.parse":
        assert result[0] == {
            "result": [
                {
                    "key": "item0001",
                    "title": "Reference Work 0001",
                    "doi": "10.5555/refkit.bench.0001",
                    "volume": "2",
                }
            ]
        }
    else:
        assert result == [
            {"result": "(Family0001, 2001)"},
            {"result": "(Family0002, 2002)"},
            {"result": "(Family0003, 2003)"},
        ]
    with pytest.raises(AssertionError, match="complete ordered output"):
        prepared.check(result[:-1])


def test_polars_worker_uses_the_declared_thread_budget(tmp_path, monkeypatch):
    from refkit_bench import runner
    from refkit_bench.report import load_result

    monkeypatch.setenv("POLARS_MAX_THREADS", "7")
    output = tmp_path / "polars"
    assert (
        runner.main(
            [
                "run",
                "--lane",
                "batch.parse",
                "--dataset",
                "tiny",
                "--package",
                "polars-eager",
                "--polars-threads",
                "2",
                "--output",
                str(output),
                "--smoke",
            ]
        )
        == 0
    )
    manifest, suite = load_result(output)
    assert manifest["checks"][0]["contract"]["polars_threads"] == 2
    benchmark = suite.get_benchmark("batch.parse/tiny/polars-eager")
    assert all(run.get_metadata()["polars_threads"] == 2 for run in benchmark.get_runs())
