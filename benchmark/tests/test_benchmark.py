from __future__ import annotations

import csv
import importlib
import importlib.metadata as metadata_module
import json
import os
from pathlib import Path

import pytest

from benchmark import adapters, fixtures, runner


def test_refkit_public_helpers_are_covered_from_benchmark_subset(tmp_path: Path) -> None:
    import refkit as rk

    path = tmp_path / "refs.bib"
    path.write_text(fixtures.audited_tiny_bibtex(), encoding="utf-8")
    style = rk.Style.load("apa")

    assert rk.cite(path, "item0001", style=style).text
    assert rk.bibliography(path, style=style).text
    assert rk.__version__ == "0.0.0"

    missing_attribute = "does_not_exist"
    with pytest.raises(AttributeError, match="does_not_exist"):
        getattr(rk, missing_attribute)


def test_refkit_version_fallback_is_covered_from_benchmark_subset(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    import refkit as rk
    import refkit._native as native

    def missing_version(name: str) -> str:
        raise metadata_module.PackageNotFoundError(name)

    with monkeypatch.context() as scoped:
        scoped.setattr(metadata_module, "version", missing_version)
        reloaded = importlib.reload(rk)

    assert reloaded.__version__ == native.__version__
    importlib.reload(rk)


def test_audited_tiny_fixture_matches_generator() -> None:
    assert fixtures.audited_tiny_bibtex() == fixtures.bibtex_for_records(
        fixtures.records_for_size("tiny")
    )


def test_materialize_workload_writes_bibtex_and_raw_inputs(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)

    assert workload.keys == ["item0001", "item0002", "item0003"]
    assert workload.family == "synthetic_scale"
    assert workload.record_count == 3
    assert workload.bibtex_path.read_text(encoding="utf-8") == workload.bibtex
    assert workload.raw_bibtex_path.read_text(encoding="utf-8") == workload.raw_bibtex
    assert workload.csl_json[0]["id"] == "item0001"
    assert "preamble" in workload.raw_bibtex.lower()
    assert workload.source_byte_count("bibtex") == len(workload.bibtex.encode("utf-8"))
    assert workload.source_text("raw_bibtex") == workload.raw_bibtex
    assert workload.source_byte_count("raw_bibtex") == len(workload.raw_bibtex.encode("utf-8"))
    assert workload.source_text("csl_json").startswith("[")
    assert len(workload.source_sha256("bibtex")) == 64
    assert workload.source_text("unknown") == ""
    assert workload.source_byte_count("unknown") == 0
    assert workload.source_sha256("unknown") == ""


def test_records_for_size_rejects_unknown_size() -> None:
    with pytest.raises(ValueError, match="unknown workload size"):
        fixtures.records_for_size("micro")


def test_fixture_sizes_are_ordered_slices_of_largest_records() -> None:
    largest = fixtures.largest_records()

    assert len(fixtures.records_for_size("tiny")) == 3
    assert len(fixtures.records_for_size("medium")) == 48
    assert len(fixtures.records_for_size("large")) == 192
    assert fixtures.records_for_size("tiny") == largest[:3]
    assert fixtures.records_for_size("medium") == largest[:48]
    assert fixtures.records_for_size("large") == largest


def test_select_cases_uses_explicit_cases_before_group() -> None:
    assert runner.select_cases(["bibtex_parse"], "render") == ["bibtex_parse"]
    assert runner.select_cases(None, "all") == list(runner.CASES)
    assert runner.select_cases(None, "render") == [
        "citation_render",
        "bibliography_render",
        "repeated_render",
    ]


def test_select_cases_rejects_unknown_group() -> None:
    with pytest.raises(SystemExit, match="unknown benchmark group"):
        runner.select_cases(None, "missing-group")


def test_select_inputs_defaults_and_deduplicates() -> None:
    assert runner.select_inputs(None) == ["tiny", "medium", "large"]
    assert runner.select_inputs(["tiny", "all", "tiny"]) == ["tiny", "medium", "large"]


def test_positive_integer_parsers_reject_invalid_values() -> None:
    assert runner.positive_int("1") == 1
    assert runner.non_negative_int("0") == 0
    with pytest.raises(Exception, match="greater than zero"):
        runner.positive_int("0")
    with pytest.raises(Exception, match="zero or greater"):
        runner.non_negative_int("-1")


def test_list_command_prints_cases(capsys: pytest.CaptureFixture[str]) -> None:
    assert runner.main(["--list"]) == 0
    out = capsys.readouterr().out
    listed_cases = [line.split("\t", maxsplit=1)[0] for line in out.splitlines()]
    assert listed_cases == [
        "bibtex_parse",
        "raw_bibtex_roundtrip",
        "citation_render",
        "bibliography_render",
        "repeated_render",
        "one_off_cite",
        "one_off_bibliography",
        "missing_reference",
        "bulk_materialization",
        "library_keys",
        "entry_lookup",
        "field_projection",
    ]
    assert "missing_reference\terror" in out
    assert "field_projection\tinspect" in out


def test_package_version_reports_missing_distribution() -> None:
    assert runner.package_version("definitely-not-installed-refkit-benchmark") == "not-installed"


def test_refkit_commit_returns_value() -> None:
    assert runner.refkit_commit()


def test_refkit_commit_handles_failure_and_empty_output(monkeypatch: pytest.MonkeyPatch) -> None:
    class Completed:
        stdout = ""

    def empty_run(*args: object, **kwargs: object) -> Completed:
        return Completed()

    def failing_run(*args: object, **kwargs: object) -> None:
        raise OSError("git unavailable")

    monkeypatch.setattr(runner.subprocess, "run", empty_run)
    assert runner.refkit_commit() == "unknown"

    monkeypatch.setattr(runner.subprocess, "run", failing_run)
    assert runner.refkit_commit() == "unknown"


def test_machine_metadata_accepts_explicit_build_mode() -> None:
    assert runner.machine_metadata("release")["build_mode"] == "release"


def test_detect_build_mode_from_native_path_and_artifacts(
    tmp_path: Path,
) -> None:
    assert runner.detect_build_mode("/tmp/target/release/_native.abi3.so") == "release"
    assert runner.detect_build_mode("/tmp/target/debug/_native.abi3.so") == "debug"
    assert runner.detect_build_mode("/tmp/refkit/_native.abi3.so", tmp_path) == "unknown"

    release = tmp_path / "target" / "release" / "lib_native.so"
    release.parent.mkdir(parents=True)
    release.write_text("release", encoding="utf-8")
    assert runner.detect_build_mode("/tmp/refkit/_native.abi3.so", tmp_path) == "unknown"

    debug_only_root = tmp_path / "debug-only"
    debug_only = debug_only_root / "target" / "debug" / "_native.dll"
    debug_only.parent.mkdir(parents=True)
    debug_only.write_text("debug", encoding="utf-8")
    assert runner.detect_build_mode("/tmp/refkit/_native.abi3.so", debug_only_root) == "unknown"

    debug = tmp_path / "target" / "debug" / "_native.pyd"
    debug.parent.mkdir(parents=True)
    debug.write_text("debug", encoding="utf-8")
    debug.touch()
    assert runner.detect_build_mode("/tmp/refkit/_native.abi3.so", tmp_path) == "unknown"

    release.touch()
    assert runner.detect_build_mode("/tmp/refkit/_native.abi3.so", tmp_path) == "unknown"
    assert runner.detect_build_mode() in {"debug", "release", "unknown"}


def test_detect_build_mode_prefers_installed_artifact_fingerprint(
    tmp_path: Path,
) -> None:
    native = tmp_path / "src" / "refkit" / "_native.abi3.so"
    release = tmp_path / "target" / "release" / "lib_native.dylib"
    debug = tmp_path / "target" / "debug" / "lib_native.so"
    native.parent.mkdir(parents=True)
    release.parent.mkdir(parents=True)
    debug.parent.mkdir(parents=True)

    native.write_text("release", encoding="utf-8")
    release.write_text("release", encoding="utf-8")
    os.utime(release, (native.stat().st_atime, native.stat().st_mtime))
    debug.write_text("debug with newer timestamp", encoding="utf-8")
    debug.touch()

    assert runner.detect_build_mode(str(native), tmp_path) == "release"
    assert not runner._same_artifact(native, debug)

    native.write_text("debug with newer timestamp", encoding="utf-8")
    os.utime(native, (debug.stat().st_atime, debug.stat().st_mtime))
    assert runner.detect_build_mode(str(native), tmp_path) == "debug"


def test_native_artifact_helpers_cover_missing_and_existing_paths(tmp_path: Path) -> None:
    candidates = runner._native_artifact_candidates(tmp_path, "release")
    assert {path.name for path in candidates} == set(runner.NATIVE_ARTIFACT_NAMES)

    candidates[0].parent.mkdir(parents=True)
    candidates[0].write_text("artifact", encoding="utf-8")
    assert candidates[0].exists()


def test_machine_metadata_contains_versions() -> None:
    metadata = runner.machine_metadata("release")

    assert metadata["build_mode"] == "release"
    assert metadata["packages"]["refkit"] == "0.0.0"
    assert metadata["packages"]["citeproc-py"] != "not-installed"
    assert metadata["packages"]["bibtexparser"] != "not-installed"


def test_adapters_prepare_supported_and_unsupported_cases(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    refkit = adapters.RefkitAdapter()
    bibtexparser = adapters.BibtexparserAdapter()

    prepared = refkit.prepare("bibtex_parse", workload, tmp_path)
    outcome = prepared.operation()
    prepared.check(outcome)
    assert outcome.count == 3

    with pytest.raises(adapters.UnsupportedOperation, match="does not render"):
        bibtexparser.prepare("citation_render", workload, tmp_path)

    with pytest.raises(adapters.UnsupportedOperation, match="does not support"):
        refkit.prepare("unknown_case", workload, tmp_path)


def test_explicit_unsupported_methods_report_reasons(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    cases_by_adapter = {
        adapters.BibtexparserAdapter(): [
            "citation_render",
            "bibliography_render",
            "repeated_render",
            "one_off_cite",
            "one_off_bibliography",
            "missing_reference",
        ],
    }

    for adapter, case_names in cases_by_adapter.items():
        for case_name in case_names:
            with pytest.raises(adapters.UnsupportedOperation):
                adapter.prepare(case_name, workload, tmp_path)


def test_check_helpers_reject_bad_outcomes(tmp_path: Path) -> None:
    tiny_records = fixtures.records_for_size("tiny")

    with pytest.raises(AssertionError, match="expected count"):
        adapters._count_is(2)(adapters.OperationOutcome("", 1))
    with pytest.raises(AssertionError, match="expected keys"):
        adapters._keys_are(["item0001"])(adapters.OperationOutcome(["item0002"], 1))
    with pytest.raises(AssertionError, match="expected count"):
        adapters._keys_are(["item0001"])(adapters.OperationOutcome(["item0001"], 0))
    with pytest.raises(AssertionError, match="expected count"):
        adapters._all_checks(adapters._count_is(2))(adapters.OperationOutcome("", 1))
    with pytest.raises(AssertionError, match="expected output"):
        adapters._text_contains("needle")(adapters.OperationOutcome("haystack", 1))
    adapters._text_contains_all(["hay", "stack"])(adapters.OperationOutcome("haystack", 1))
    with pytest.raises(AssertionError, match="expected output"):
        adapters._text_contains_all(["needle"])(adapters.OperationOutcome("haystack", 1))
    adapters._citation_output_matches(tiny_records[:1])(
        adapters.OperationOutcome("(Family0001, 2001)", 1)
    )
    with pytest.raises(AssertionError, match="rendered citations"):
        adapters._citation_output_matches(tiny_records[:2])(
            adapters.OperationOutcome("(Family0002, 2002)\n(Family0001, 2001)", 2)
        )
    bibliography_row = (
        "Family0001, G. (2001). Reference Work 0001. "
        "Journal of Citation Benchmarks, 2, 3-11. "
        "https://doi.org/10.5555/refkit.bench.0001"
    )
    adapters._bibliography_output_matches(tiny_records[:1])(
        adapters.OperationOutcome(bibliography_row, 1)
    )
    with pytest.raises(AssertionError, match="bibliography rows"):
        adapters._bibliography_output_matches(tiny_records[:1])(adapters.OperationOutcome("", 0))
    with pytest.raises(AssertionError, match="Reference Work 0001"):
        adapters._bibliography_output_matches(tiny_records[:1])(
            adapters.OperationOutcome("Family0001, G. (2001).", 1)
        )
    with pytest.raises(AssertionError, match="expected detail"):
        adapters._detail_contains("needle")(adapters.OperationOutcome("", 1, "haystack"))
    path = Path("missing-output.bib")
    with pytest.raises(FileNotFoundError):
        adapters._raw_roundtrip_check(["item0001"])(adapters.OperationOutcome(path, 1))
    wrong = tmp_path / "wrong.bib"
    wrong.write_text("haystack", encoding="utf-8")
    with pytest.raises(AssertionError, match="expected written file"):
        adapters._raw_roundtrip_check(["item0001"])(adapters.OperationOutcome(wrong, 1))
    partial = tmp_path / "partial.bib"
    partial.write_text(
        "Edited Benchmark Title benchmark fixture with raw BibTeX blocks "
        "benchjournal Reference benchmark fixture item0001",
        encoding="utf-8",
    )
    with pytest.raises(AssertionError, match="expected 1 written entries"):
        adapters._raw_roundtrip_check(["item0001"])(adapters.OperationOutcome(partial, 1))
    with pytest.raises(AssertionError, match="projected rows"):
        adapters._projection_contains(fixtures.records_for_size("tiny"))(
            adapters.OperationOutcome([], 0)
        )
    with pytest.raises(AssertionError, match="expected title"):
        adapters._projection_contains(fixtures.records_for_size("tiny")[:1])(
            adapters.OperationOutcome([{"key": "item0001", "title": "wrong"}], 1)
        )
    with pytest.raises(AssertionError, match="to include 'doi'"):
        adapters._projection_contains(
            fixtures.records_for_size("tiny")[:1],
            required_fields=("key", "title", "doi", "volume"),
        )(adapters.OperationOutcome([{"key": "item0001", "title": "Reference Work 0001"}], 1))
    with pytest.raises(AssertionError, match="projected rows to contain"):
        adapters._projection_contains(fixtures.records_for_size("tiny")[:1])(
            adapters.OperationOutcome([{"key": "item0002", "title": "Reference Work 0002"}], 1)
        )
    with pytest.raises(AssertionError, match="expected DOI"):
        adapters._projection_contains(fixtures.records_for_size("tiny")[:1])(
            adapters.OperationOutcome(
                [
                    {
                        "key": "item0001",
                        "title": "Reference Work 0001",
                        "doi": "wrong",
                    }
                ],
                1,
            )
        )
    with pytest.raises(AssertionError, match="expected DOI"):
        adapters._projection_contains(
            fixtures.records_for_size("tiny")[:1],
            required_fields=("key", "title", "doi", "volume"),
        )(
            adapters.OperationOutcome(
                [
                    {
                        "key": "item0001",
                        "title": "Reference Work 0001",
                        "doi": None,
                        "volume": "1",
                    }
                ],
                1,
            )
        )
    with pytest.raises(AssertionError, match="expected volume"):
        adapters._projection_contains(fixtures.records_for_size("tiny")[:1])(
            adapters.OperationOutcome(
                [
                    {
                        "key": "item0001",
                        "title": "Reference Work 0001",
                        "volume": "wrong",
                    }
                ],
                1,
            )
        )
    with pytest.raises(AssertionError, match="expected volume"):
        adapters._projection_contains(
            fixtures.records_for_size("tiny")[:1],
            required_fields=("key", "title", "doi", "volume"),
        )(
            adapters.OperationOutcome(
                [
                    {
                        "key": "item0001",
                        "title": "Reference Work 0001",
                        "doi": fixtures.records_for_size("tiny")[0].doi,
                        "volume": None,
                    }
                ],
                1,
            )
        )
    with pytest.raises(AssertionError, match="expected 1 entries"):
        adapters._entries_match(fixtures.records_for_size("tiny")[:1])(
            adapters.OperationOutcome([], 0)
        )
    with pytest.raises(AssertionError, match="expected entry key"):
        adapters._entries_match(fixtures.records_for_size("tiny")[:1])(
            adapters.OperationOutcome([{"ID": "wrong", "title": "Reference Work 0001"}], 1)
        )
    with pytest.raises(AssertionError, match="expected title"):
        adapters._entries_match(fixtures.records_for_size("tiny")[:1])(
            adapters.OperationOutcome([{"ID": "item0001", "title": "wrong"}], 1)
        )
    assert adapters._first(["title"]) == "title"
    assert adapters._first([]) is None


def test_each_comparable_workflow_has_correctness_check(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    cases_by_adapter = {
        adapters.RefkitAdapter(): [
            "bibtex_parse",
            "raw_bibtex_roundtrip",
            "citation_render",
            "bibliography_render",
            "repeated_render",
            "one_off_cite",
            "one_off_bibliography",
            "missing_reference",
            "bulk_materialization",
            "library_keys",
            "entry_lookup",
            "field_projection",
        ],
        adapters.CiteprocPyAdapter(): [
            "bibtex_parse",
            "citation_render",
            "bibliography_render",
            "repeated_render",
            "one_off_cite",
            "one_off_bibliography",
            "missing_reference",
            "bulk_materialization",
            "library_keys",
            "entry_lookup",
            "field_projection",
        ],
        adapters.BibtexparserAdapter(): [
            "bibtex_parse",
            "raw_bibtex_roundtrip",
            "bulk_materialization",
            "library_keys",
            "entry_lookup",
            "field_projection",
        ],
    }

    for adapter, case_names in cases_by_adapter.items():
        for case_name in case_names:
            prepared = adapter.prepare(case_name, workload, tmp_path)
            outcome = prepared.operation()
            prepared.check(outcome)
            assert outcome.count >= 1


def test_benchmark_render_metadata_describes_citation_requests(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    refkit_bibliography = adapters.RefkitAdapter().prepare(
        "bibliography_render", workload, tmp_path
    )
    refkit_one_off = adapters.RefkitAdapter().prepare("one_off_bibliography", workload, tmp_path)
    citeproc_bibliography = adapters.CiteprocPyAdapter().prepare(
        "bibliography_render", workload, tmp_path
    )
    citeproc_missing = adapters.CiteprocPyAdapter().prepare("missing_reference", workload, tmp_path)

    assert refkit_bibliography.metadata["citation_count"] == 0
    assert refkit_one_off.metadata["citation_count"] == 0
    assert citeproc_bibliography.metadata["citation_count"] == len(workload.records)
    with pytest.raises(AssertionError, match="expected count 1"):
        citeproc_missing.check(
            adapters.OperationOutcome("(missing-reference?)", 2, "missing-reference")
        )


def test_repeated_render_uses_full_selected_input(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("medium", tmp_path)

    for adapter in (adapters.RefkitAdapter(), adapters.CiteprocPyAdapter()):
        prepared = adapter.prepare("repeated_render", workload, tmp_path)
        outcome = prepared.operation()

        prepared.check(outcome)
        assert outcome.count == len(workload.records)
        assert prepared.metadata["citation_count"] == len(workload.records)


def test_citeproc_bulk_materialization_uses_bibtex_source(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    prepared = adapters.CiteprocPyAdapter().prepare("bulk_materialization", workload, tmp_path)

    workload.bibtex_path.write_text("", encoding="utf-8")
    outcome = prepared.operation()

    assert outcome.count == 3


def test_run_adapter_case_emits_unsupported_rows(tmp_path: Path) -> None:
    metadata = runner.machine_metadata("release")
    workload = fixtures.materialize_workload("tiny", tmp_path)
    rows = runner.run_adapter_case(
        adapter=adapters.BibtexparserAdapter(),
        case=runner.CASES["citation_render"],
        workload=workload,
        directory=tmp_path,
        rounds=2,
        warmups=1,
        metadata=metadata,
    )

    assert len(rows) == 1
    assert rows[0]["status"] == "unsupported"
    assert rows[0]["seconds"] == 0.0


def test_run_adapter_case_emits_failed_setup_rows(
    tmp_path: Path,
) -> None:
    class BrokenAdapter(adapters.PackageAdapter):
        name = "broken"
        distribution = "broken"

        def prepare_bibtex_parse(self, workload: object, directory: Path) -> object:
            raise RuntimeError("setup failed")

    metadata = runner.machine_metadata("release")
    workload = fixtures.materialize_workload("tiny", tmp_path)
    rows = runner.run_adapter_case(
        adapter=BrokenAdapter(),
        case=runner.CASES["bibtex_parse"],
        workload=workload,
        directory=tmp_path,
        rounds=2,
        warmups=1,
        metadata=metadata,
    )

    assert rows[0]["status"] == "failed"
    assert rows[0]["phase"] == "setup"


def test_run_adapter_case_emits_failed_execution_rows(
    tmp_path: Path,
) -> None:
    class FailingAdapter(adapters.PackageAdapter):
        name = "failing"
        distribution = "failing"

        def prepare_bibtex_parse(
            self,
            workload: object,
            directory: Path,
        ) -> adapters.PreparedOperation:
            return adapters.PreparedOperation(
                phase="parse",
                operation=lambda: adapters.OperationOutcome("wrong", 0),
                check=lambda outcome: (_ for _ in ()).throw(AssertionError("bad count")),
            )

    metadata = runner.machine_metadata("release")
    workload = fixtures.materialize_workload("tiny", tmp_path)
    rows = runner.run_adapter_case(
        adapter=FailingAdapter(),
        case=runner.CASES["bibtex_parse"],
        workload=workload,
        directory=tmp_path,
        rounds=2,
        warmups=0,
        metadata=metadata,
    )

    assert len(rows) == 1
    assert rows[0]["status"] == "failed"
    assert rows[0]["phase"] == "parse"


def test_run_adapter_case_emits_failed_warmup_rows(
    tmp_path: Path,
) -> None:
    class WarmupFailingAdapter(adapters.PackageAdapter):
        name = "warmup-failing"
        distribution = "warmup-failing"

        def prepare_bibtex_parse(
            self,
            workload: object,
            directory: Path,
        ) -> adapters.PreparedOperation:
            return adapters.PreparedOperation(
                phase="parse",
                operation=lambda: adapters.OperationOutcome("wrong", 0),
                check=lambda outcome: (_ for _ in ()).throw(AssertionError("warmup failed")),
            )

    metadata = runner.machine_metadata("release")
    rows = runner.run_adapter_case(
        adapter=WarmupFailingAdapter(),
        case=runner.CASES["bibtex_parse"],
        workload=fixtures.materialize_workload("tiny", tmp_path),
        directory=tmp_path,
        rounds=2,
        warmups=1,
        metadata=metadata,
    )

    assert len(rows) == 1
    assert rows[0]["phase"] == "parse"
    assert rows[0]["operation_phase"] == "parse"
    assert rows[0]["round"] == 0
    assert rows[0]["seconds"] == 0.0
    assert rows[0]["status"] == "failed"
    assert rows[0]["detail"] == "AssertionError('warmup failed')"
    assert isinstance(rows[0]["setup_seconds"], float)
    assert rows[0]["setup_seconds"] >= 0.0
    assert rows[0]["source_format"] == "unknown"


def test_run_suite_writes_ok_and_unsupported_rows() -> None:
    result = runner.run_suite(
        case_names=["bibtex_parse", "citation_render"],
        input_sizes=["tiny"],
        rounds=1,
        warmups=1,
        build_mode="release",
    )
    rows = result["rows"]

    assert isinstance(result["metadata"], dict)
    assert any(row["status"] == "ok" for row in rows)
    assert any(row["status"] == "unsupported" for row in rows)
    assert all(row["input"] == "tiny" for row in rows)
    for row in rows:
        assert set(runner.RESULT_FIELDS) <= set(row)
        assert row["input_size"] == "tiny"
        assert row["workload_family"] == "synthetic_scale"
        assert row["record_count"] == 3
        assert row["rounds"] == 1
        assert row["warmups"] == 1
        assert isinstance(row["setup_seconds"], float)
        assert row["setup_seconds"] >= 0.0
        assert row["adapter_version"] == row["package_version"]


def test_write_json_and_csv_outputs(tmp_path: Path) -> None:
    result = runner.run_suite(
        case_names=["bibtex_parse"],
        input_sizes=["tiny"],
        rounds=1,
        warmups=0,
        build_mode="release",
    )
    json_path = tmp_path / "nested" / "result.json"
    csv_path = tmp_path / "nested" / "result.csv"

    runner.write_json(json_path, result)
    runner.write_csv(csv_path, result["rows"])

    loaded = json.loads(json_path.read_text(encoding="utf-8"))
    with csv_path.open(encoding="utf-8") as handle:
        csv_rows = list(csv.DictReader(handle))

    assert len(loaded["rows"]) == len(result["rows"])
    assert csv_rows[0]["case"] == "bibtex_parse"
    assert csv_rows[0]["workload_family"] == "synthetic_scale"
    assert csv_rows[0]["record_count"] == "3"
    assert csv_rows[0]["source_format"] == "bibtex"
    assert csv_rows[0]["input_sha256"]


def test_main_runs_case_and_writes_outputs(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    json_path = tmp_path / "result.json"
    csv_path = tmp_path / "result.csv"

    exit_code = runner.main(
        [
            "--case",
            "bibtex_parse",
            "--input",
            "tiny",
            "--rounds",
            "1",
            "--warmups",
            "0",
            "--build-mode",
            "release",
            "--json",
            str(json_path),
            "--csv",
            str(csv_path),
        ]
    )

    assert exit_code == 0
    assert json_path.exists()
    assert csv_path.exists()
    assert '"rows": 3' in capsys.readouterr().out


def test_main_returns_failure_for_failed_rows(
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    def fake_run_suite(**kwargs: object) -> dict[str, object]:
        return {
            "metadata": {},
            "rows": [
                {
                    "status": "failed",
                    "case": "bibtex_parse",
                    "group": "parse",
                    "package": "fake",
                    "package_version": "0",
                    "phase": "parse",
                    "input": "tiny",
                    "round": 1,
                    "seconds": 0.0,
                    "detail": "failure",
                    "python": "3",
                    "os": "test",
                    "cpu": "test",
                    "refkit_version": "0.0.0",
                    "refkit_commit": "test",
                    "build_mode": "release",
                }
            ],
        }

    monkeypatch.setattr(runner, "run_suite", fake_run_suite)

    assert runner.main(["--case", "bibtex_parse", "--input", "tiny"]) == 1
    assert '"failed": 1' in capsys.readouterr().out
