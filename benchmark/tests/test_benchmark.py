from __future__ import annotations

import csv
import importlib.metadata as metadata_module
import json
import os
from dataclasses import replace
from pathlib import Path
from typing import Any, cast

import pytest

from refkit_bench import adapters, fixtures, runner
from refkit_bench._adapters import bibtexparser_v2 as bibtexparser_v2_adapter


def run_prepared(prepared: adapters.PreparedOperation) -> adapters.OperationOutcome:
    try:
        outcome = prepared.operation()
        prepared.check(outcome)
        return outcome
    finally:
        prepared.cleanup()


def row_string(row: runner.Row, key: str) -> str:
    return str(row[key])


def assert_prepared_fails_as_benchmark_row(
    prepared: adapters.PreparedOperation,
    workload: fixtures.Workload,
    tmp_path: Path,
) -> None:
    class StaticAdapter(adapters.PackageAdapter):
        name = "static"
        distribution = "static"

        def prepare_parse_bibtex(
            self,
            workload: fixtures.Workload,
            directory: Path,
        ) -> adapters.PreparedOperation:
            return prepared

    rows = runner.run_adapter_lane(
        adapter=StaticAdapter(),
        lane=runner.LANES["input.bibtex"],
        participant=runner.participant("static", "parse_bibtex"),
        workload=workload,
        directory=tmp_path,
        rounds=1,
        warmups=0,
        metadata=runner.machine_metadata("release"),
    )

    assert len(rows) == 1
    assert rows[0]["status"] == "failed"


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
    assert workload.dirty_bibtex_path.read_text(encoding="utf-8") == workload.dirty_bibtex
    assert workload.duplicate_bibtex_path.read_text(encoding="utf-8") == workload.duplicate_bibtex
    assert workload.csl_json[0]["id"] == "item0001"
    assert "preamble" in workload.raw_bibtex.lower()
    assert "No close" in workload.dirty_bibtex
    assert workload.duplicate_entry_key == "item0001"
    assert workload.duplicate_field_key == "item0002"
    assert "Duplicate benchmark entry" in workload.duplicate_bibtex
    assert "Duplicate benchmark field" in workload.duplicate_bibtex
    assert workload.source_byte_count("bibtex") == len(workload.bibtex.encode("utf-8"))
    assert workload.source_text("raw_bibtex") == workload.raw_bibtex
    assert workload.source_byte_count("raw_bibtex") == len(workload.raw_bibtex.encode("utf-8"))
    assert workload.source_text("dirty_bibtex") == workload.dirty_bibtex
    assert workload.source_byte_count("dirty_bibtex") == len(workload.dirty_bibtex.encode("utf-8"))
    assert workload.source_text("duplicate_bibtex") == workload.duplicate_bibtex
    assert workload.source_byte_count("duplicate_bibtex") == len(
        workload.duplicate_bibtex.encode("utf-8")
    )
    assert workload.source_name("bibtex") == "synthetic_scale:tiny:bibtex"
    assert workload.source_path("bibtex") == str(workload.bibtex_path)
    assert workload.source_path("duplicate_bibtex") == str(workload.duplicate_bibtex_path)
    assert workload.source_license("bibtex") == "Apache-2.0"
    assert workload.source_text("csl_json").startswith("[")
    assert len(workload.source_sha256("bibtex")) == 64
    assert workload.source_text("unknown") == ""
    assert workload.source_name("unknown") == ""
    assert workload.source_path("unknown") == ""
    assert workload.source_license("unknown") == ""
    assert workload.source_byte_count("unknown") == 0
    assert workload.source_sha256("unknown") == ""


def test_materialize_arxiv_workload_uses_checked_in_real_bibtex(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("arxiv", tmp_path)

    assert workload.family == "arxiv_wild_subset"
    assert workload.record_count == 12
    assert workload.keys[:3] == ["ijcai2019p684", "10.1145/3325887", "Kimi_K2.5"]
    assert workload.bibtex.startswith("% Real BibTeX subset")
    assert "DeepResearchGym" in workload.bibtex
    assert "Ancient–Modern Chinese Translation" in workload.bibtex
    assert workload.bibtex_path.read_text(encoding="utf-8") == workload.bibtex
    assert workload.source_name("bibtex") == "arxiv_wild_subset:arxiv:bibtex"
    assert workload.source_license("bibtex") == "mixed-arxiv-source-licenses"
    assert workload.source_byte_count("bibtex") == len(workload.bibtex.encode("utf-8"))
    assert len(workload.source_sha256("bibtex")) == 64
    assert workload.duplicate_entry_key == "ijcai2019p684"
    assert workload.duplicate_field_key == "10.1145/3325887"
    assert "Duplicate benchmark field" in workload.source_text("duplicate_bibtex")
    assert workload.csl_json[0]["id"] == "ijcai2019p684"
    assert workload.csl_json[0]["type"] == "paper-conference"


def test_arxiv_fixture_falls_back_to_packaged_data(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
) -> None:
    packaged = tmp_path / "references-subset.bib"
    packaged.write_text("% packaged arxiv fixture\n", encoding="utf-8")

    monkeypatch.setattr(fixtures, "ARXIV_SUBSET_PATH", tmp_path / "missing-repo.bib")
    monkeypatch.setattr(fixtures, "PACKAGED_ARXIV_SUBSET_PATH", packaged)

    assert fixtures.arxiv_subset_path() == packaged
    assert fixtures.arxiv_bibtex() == "% packaged arxiv fixture\n"


def test_arxiv_fixture_reports_missing_data(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
) -> None:
    monkeypatch.setattr(fixtures, "ARXIV_SUBSET_PATH", tmp_path / "missing-repo.bib")
    monkeypatch.setattr(fixtures, "PACKAGED_ARXIV_SUBSET_PATH", tmp_path / "missing-package.bib")

    with pytest.raises(FileNotFoundError, match="arxiv workload fixture is missing"):
        fixtures.arxiv_subset_path()


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


def test_dirty_bibtex_for_empty_records_contains_only_malformed_block() -> None:
    dirty = fixtures.dirty_bibtex_for_records(())

    assert dirty.count("@") == 1
    assert dirty.lstrip().startswith("@broken{")
    assert "No close" in dirty


def test_duplicate_bibtex_requires_three_records() -> None:
    with pytest.raises(ValueError, match="at least three records"):
        fixtures.duplicate_bibtex_for_records(fixtures.records_for_size("tiny")[:2])


def test_record_source_forms_omit_absent_optional_fields() -> None:
    record = fixtures.Record(
        key="minimal",
        family="Solo",
        given="Sam",
        title="Minimal Reference",
        year=2024,
        volume=None,
        page_start=None,
        page_end=None,
        doi=None,
        container="",
    )

    bibtex = fixtures.bibtex_for_records((record,))
    csl = fixtures.csl_json_for_records((record,))[0]

    assert "volume =" not in bibtex
    assert "pages =" not in bibtex
    assert "doi =" not in bibtex
    assert "volume" not in csl
    assert "page" not in csl
    assert "DOI" not in csl
    assert "container-title" not in csl


def test_select_lanes_uses_explicit_lanes_before_group() -> None:
    assert runner.select_lanes(["input.bibtex"], "render.prepared") == ["input.bibtex"]
    all_lanes = runner.select_lanes(None, "all")
    assert "input.bibtex-text" in all_lanes
    assert "input.bibtex" in all_lanes
    assert "render.prepared-citation" in all_lanes
    assert len(all_lanes) == len(set(all_lanes))
    assert runner.select_lanes(None, "render.prepared") == [
        "render.prepared-citation",
        "render.prepared-bibliography",
        "render.cited-bibliography",
        "render.repeated-citations",
    ]


def test_select_lanes_rejects_unknown_group() -> None:
    with pytest.raises(SystemExit, match="unknown benchmark lane group"):
        runner.select_lanes(None, "missing-group")


def test_select_inputs_defaults_and_deduplicates() -> None:
    assert runner.select_inputs(None) == ["tiny", "medium", "large", "arxiv"]
    assert runner.select_inputs(["tiny", "all", "tiny"]) == [
        "tiny",
        "medium",
        "large",
        "arxiv",
    ]


def test_positive_integer_parsers_reject_invalid_values() -> None:
    assert runner.positive_int("1") == 1
    assert runner.non_negative_int("0") == 0
    with pytest.raises(Exception, match="greater than zero"):
        runner.positive_int("0")
    with pytest.raises(Exception, match="zero or greater"):
        runner.non_negative_int("-1")


def test_list_command_prints_lanes(capsys: pytest.CaptureFixture[str]) -> None:
    assert runner.main(["--list"]) == 0
    out = capsys.readouterr().out
    rows = {line.split("\t")[0]: line.split("\t") for line in out.splitlines()}

    assert all(len(row) == 6 for row in rows.values())
    assert rows["input.bibtex"][1:5] == [
        "input.normalized",
        "normalized_bibliography_input",
        "bibtex_input",
        "refkit,polars-refkit,bibtexparser-2.x,pybtex",
    ]
    assert rows["input.bibtex-text"][1:5] == [
        "input.normalized",
        "normalized_bibliography_input",
        "bibtex_text_input",
        "refkit,bibtexparser-2.x,pybtex",
    ]
    assert rows["input.diagnostics"][1:5] == [
        "input.normalized",
        "diagnostic_reporting",
        "dirty_bibtex_diagnostics",
        "refkit,bibtexparser-2.x",
    ]
    assert rows["raw-bibtex.blocks"][1:5] == [
        "raw-bibtex",
        "raw_bibtex_document",
        "raw_block_materialization",
        "refkit,bibtexparser-2.x",
    ]
    assert rows["raw-bibtex.duplicates"][1:5] == [
        "raw-bibtex",
        "raw_bibtex_document",
        "duplicate_handling",
        "refkit,bibtexparser-2.x",
    ]
    assert rows["style.load"][1:5] == [
        "style",
        "citation_style_input",
        "style_resolution",
        "refkit,citeproc-py",
    ]
    assert rows["errors.missing-reference"][1:5] == [
        "errors",
        "error_contracts",
        "missing_reference",
        "refkit,citeproc-py",
    ]
    assert rows["inspect.fields"][1:5] == [
        "inspect.entries",
        "entry_inspection",
        "field_projection",
        "refkit,bibtexparser-2.x,pybtex",
    ]
    assert rows["bulk.polars.fields"][1:5] == [
        "bulk.polars",
        "bulk_tabular_processing",
        "tabular_field_projection",
        "polars-refkit",
    ]
    assert all(row[5] for row in rows.values())


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


def test_machine_metadata_contains_versions() -> None:
    metadata = runner.machine_metadata("release")

    assert metadata["build_mode"] == "release"
    assert metadata["packages"]["refkit"] == metadata_module.version("refkit")
    assert metadata["packages"]["polars-refkit"] == metadata_module.version("polars-refkit")
    assert metadata["packages"]["citeproc-py"] != "not-installed"
    assert metadata["packages"]["bibtexparser"] != "not-installed"
    assert metadata["packages"]["bibtexparser-v2"] == metadata["packages"]["bibtexparser"]
    assert metadata["packages"]["pybtex"] != "not-installed"


def test_adapter_registry_contains_current_benchmark_packages() -> None:
    names = [adapter.name for adapter in adapters.adapters()]

    assert set(names) == {
        "refkit",
        "polars-refkit",
        "citeproc-py",
        "bibtexparser-2.x",
        "pybtex",
    }


def test_adapters_prepare_supported_lane_operations(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    refkit = adapters.RefkitAdapter()
    polars_refkit = adapters.PolarsRefkitAdapter()
    bibtexparser_v2 = adapters.BibtexparserV2Adapter()
    pybtex = adapters.PybtexAdapter()

    prepared = refkit.prepare("parse_bibtex", workload, tmp_path)
    outcome = run_prepared(prepared)
    assert outcome.count == 3

    prepared = pybtex.prepare("parse_bibtex_text", workload, tmp_path)
    outcome = run_prepared(prepared)
    assert outcome.count == 3

    prepared = polars_refkit.prepare("project_fields_eager", workload, tmp_path)
    outcome = run_prepared(prepared)
    assert outcome.count == 3

    prepared = polars_refkit.prepare("render_citation_expression_eager", workload, tmp_path)
    outcome = run_prepared(prepared)
    assert outcome.count == 1

    prepared = polars_refkit.prepare("render_bibliography_expression_eager", workload, tmp_path)
    outcome = run_prepared(prepared)
    assert outcome.count == 3

    prepared = polars_refkit.prepare("render_citation_each_eager", workload, tmp_path)
    outcome = run_prepared(prepared)
    assert outcome.count == 3

    prepared = refkit.prepare("extract_diagnostics", workload, tmp_path)
    outcome = run_prepared(prepared)
    assert outcome.count == 4

    prepared = refkit.prepare("materialize_raw_blocks", workload, tmp_path)
    outcome = run_prepared(prepared)
    assert outcome.count >= len(workload.records)

    prepared = bibtexparser_v2.prepare("handle_duplicates", workload, tmp_path)
    outcome = run_prepared(prepared)
    assert outcome.count == 2

    with pytest.raises(adapters.MissingBenchmarkOperation, match="no benchmark operation"):
        bibtexparser_v2.prepare("render_one_prepared_citation", workload, tmp_path)

    with pytest.raises(adapters.MissingBenchmarkOperation, match="no benchmark operation"):
        pybtex.prepare("render_one_prepared_citation", workload, tmp_path)

    with pytest.raises(adapters.MissingBenchmarkOperation, match="no benchmark operation"):
        refkit.prepare("unknown_operation", workload, tmp_path)


def test_bibtexparser_v2_adapter_requires_requested_beta(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    adapter = bibtexparser_v2_adapter.BibtexparserV2Adapter()

    monkeypatch.setattr(bibtexparser_v2_adapter, "package_version", lambda name: "2.0.0b8")

    with pytest.raises(RuntimeError, match="bibtexparser==2.0.0b9"):
        adapter.prepare("parse_bibtex", workload, tmp_path)


def test_bibtexparser_v2_raw_edit_and_projection_behaviour(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    adapter = adapters.BibtexparserV2Adapter()

    roundtrip = run_prepared(adapter.prepare("roundtrip_raw_bibtex_edit", workload, tmp_path))
    written = Path(str(roundtrip.value)).read_text(encoding="utf-8")
    fields = run_prepared(adapter.prepare("project_fields", workload, tmp_path))

    assert "Edited Benchmark Title" in written
    assert "benchjournal" in written
    assert workload.keys[0] in written
    rows = {str(row["key"]): row for row in cast(list[dict[str, Any]], fields.value)}

    assert rows["item0001"]["title"] == "Reference Work 0001"
    assert rows["item0001"]["doi"] == "10.5555/refkit.bench.0001"
    assert rows["item0001"]["volume"] == "2"


def test_raw_block_and_duplicate_lanes_expose_public_signals(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)

    for adapter in (adapters.RefkitAdapter(), adapters.BibtexparserV2Adapter()):
        raw_blocks = run_prepared(adapter.prepare("materialize_raw_blocks", workload, tmp_path))
        duplicate_signals = run_prepared(adapter.prepare("handle_duplicates", workload, tmp_path))

        block_rows = cast(list[dict[str, Any]], raw_blocks.value)
        duplicate_rows = cast(list[dict[str, Any]], duplicate_signals.value)
        assert {row["kind"] for row in block_rows} >= {"comment", "entry"}
        assert {row["key"] for row in block_rows if row["kind"] == "entry"} >= set(workload.keys)
        assert {(row["kind"], row["key"], row["field"]) for row in duplicate_rows} == {
            ("duplicate_entry", "item0001", ""),
            ("duplicate_field", "item0002", "title"),
        }

    arxiv = fixtures.materialize_workload("arxiv", tmp_path)
    arxiv_blocks = run_prepared(
        adapters.RefkitAdapter().prepare("materialize_raw_blocks", arxiv, tmp_path)
    )
    assert arxiv_blocks.count >= len(arxiv.records)


def test_bibtexparser_raw_block_lane_reports_failed_blocks(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    raw_with_failure = replace(
        workload,
        raw_bibtex=workload.raw_bibtex + "\n@broken{bad,\n  title = {No close}\n",
    )
    prepared = adapters.BibtexparserV2Adapter().prepare(
        "materialize_raw_blocks",
        raw_with_failure,
        tmp_path,
    )
    outcome = run_prepared(prepared)
    rows = cast(list[dict[str, Any]], outcome.value)

    assert any(row["kind"] == "failed" for row in rows)


def test_bibtexparser_duplicate_lane_ignores_unrelated_parse_failures(
    tmp_path: Path,
) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    duplicate_with_failure = replace(
        workload,
        duplicate_bibtex=workload.duplicate_bibtex + "\n@broken{bad,\n  title = {No close}\n",
    )
    prepared = adapters.BibtexparserV2Adapter().prepare(
        "handle_duplicates",
        duplicate_with_failure,
        tmp_path,
    )
    outcome = run_prepared(prepared)
    rows = cast(list[dict[str, Any]], outcome.value)

    assert {row["kind"] for row in rows} == {"duplicate_entry", "duplicate_field"}


def test_diagnostics_lane_materializes_dirty_parse_reports(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    refkit = run_prepared(
        adapters.RefkitAdapter().prepare("extract_diagnostics", workload, tmp_path)
    )
    bibtexparser = run_prepared(
        adapters.BibtexparserV2Adapter().prepare("extract_diagnostics", workload, tmp_path)
    )

    refkit_rows = cast(list[dict[str, str]], refkit.value)
    bibtexparser_rows = cast(list[dict[str, str]], bibtexparser.value)

    assert refkit.count == 4
    assert bibtexparser.count == 2
    assert any("duplicate" in row["message"] for row in refkit_rows)
    assert {row["kind"] for row in bibtexparser_rows} == {
        "DuplicateBlockKeyBlock",
        "ParsingFailedBlock",
    }


def test_bibtexparser_v2_adapter_handles_missing_optional_fields(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    bibtex_path = tmp_path / "missing-optional.bib"
    bibtex = """@article{item0001,
  author = {Family0001, Given0001},
  title = {Reference Work 0001},
  year = {2001}
}
"""
    bibtex_path.write_text(bibtex, encoding="utf-8")
    missing_optional = replace(workload, bibtex=bibtex, bibtex_path=bibtex_path)
    prepared = adapters.BibtexparserV2Adapter().prepare(
        "project_fields",
        missing_optional,
        tmp_path,
    )

    outcome = prepared.operation()
    rows = list(cast(list[dict[str, Any]], outcome.value))

    assert rows[0]["key"] == "item0001"
    assert rows[0]["title"] == "Reference Work 0001"
    assert rows[0]["doi"] is None
    assert rows[0]["volume"] is None


def test_pybtex_adapter_parse_and_inspection_contracts(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    adapter = adapters.PybtexAdapter()

    for operation in ("parse_bibtex", "parse_bibtex_text", "list_keys"):
        outcome = run_prepared(adapter.prepare(operation, workload, tmp_path))
        assert outcome.count == len(workload.records)

    lookup = run_prepared(adapter.prepare("lookup_entries", workload, tmp_path))
    fields = run_prepared(adapter.prepare("project_fields", workload, tmp_path))

    lookup_rows = cast(list[dict[str, Any]], lookup.value)
    field_rows = {str(row["key"]): row for row in cast(list[dict[str, Any]], fields.value)}

    assert lookup_rows[0] == {"key": "item0001", "title": "Reference Work 0001"}
    assert field_rows["item0001"]["doi"] == "10.5555/refkit.bench.0001"
    assert field_rows["item0001"]["volume"] == "2"


def test_pybtex_adapter_projects_missing_optional_fields(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    bibtex = """@article{item0001,
  author = {Family0001, Given0001},
  title = {Reference Work 0001},
  year = {2001}
}
"""
    missing_optional = replace(workload, bibtex=bibtex)
    prepared = adapters.PybtexAdapter().prepare("project_fields", missing_optional, tmp_path)

    outcome = prepared.operation()
    rows = list(cast(list[dict[str, Any]], outcome.value))

    assert rows[0]["key"] == "item0001"
    assert rows[0]["title"] == "Reference Work 0001"
    assert rows[0]["doi"] is None
    assert rows[0]["volume"] is None


def test_bibtexparser_v2_raw_edit_adds_missing_title_field(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    raw_bibtex = """% benchmark fixture with raw BibTeX blocks
@string{benchjournal = {Journal of Citation Benchmarks}}
@preamble{Reference benchmark fixture}

@article{item0001,
  journal = benchjournal,
  year = {2001}
}
"""
    missing_title = replace(workload, raw_bibtex=raw_bibtex)
    prepared = adapters.BibtexparserV2Adapter().prepare(
        "roundtrip_raw_bibtex_edit",
        missing_title,
        tmp_path,
    )

    outcome = prepared.operation()
    written = Path(str(outcome.value)).read_text(encoding="utf-8")

    assert "Edited Benchmark Title" in written
    assert "item0001" in written


def test_bibtexparser_v2_recovery_check_rejects_unexpected_failed_blocks(
    tmp_path: Path,
) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    dirty_path = tmp_path / "extra-dirty.bib"
    dirty_path.write_text(
        workload.dirty_bibtex + "\n\n@broken{extra,\n  title = {Extra}\n",
        encoding="utf-8",
    )
    dirty = replace(
        workload,
        dirty_bibtex=dirty_path.read_text(encoding="utf-8"),
        dirty_bibtex_path=dirty_path,
    )
    prepared = adapters.BibtexparserV2Adapter().prepare("recover_dirty_bibtex", dirty, tmp_path)
    outcome = prepared.operation()

    with pytest.raises(AssertionError, match="failed block signatures"):
        prepared.check(outcome)


def test_benchmark_reports_failed_rows_for_invalid_key_outputs(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    prepared = adapters.RefkitAdapter().prepare("list_keys", workload, tmp_path)

    assert_prepared_fails_as_benchmark_row(
        replace(prepared, operation=lambda: adapters.OperationOutcome(["item0002"], 3)),
        workload,
        tmp_path,
    )
    assert_prepared_fails_as_benchmark_row(
        replace(prepared, operation=lambda: adapters.OperationOutcome(workload.keys, 0)),
        workload,
        tmp_path,
    )


def test_benchmark_reports_failed_rows_for_invalid_projection_outputs(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    records = workload.records
    prepared = adapters.RefkitAdapter().prepare("project_fields", workload, tmp_path)
    valid_rows = [
        {
            "key": record.key,
            "title": record.title,
            "doi": record.doi,
            "volume": str(record.volume),
        }
        for record in records
    ]
    invalid_outputs = [
        [],
        [
            {
                **row,
                "key": f"missing-{index}",
            }
            for index, row in enumerate(valid_rows)
        ],
        [{"key": record.key, "title": record.title} for record in records],
        [{**row, "title": "wrong"} for row in valid_rows],
        [{**row, "doi": None} for row in valid_rows],
        [{**row, "doi": "wrong"} for row in valid_rows],
        [{**row, "volume": "wrong"} for row in valid_rows],
    ]

    for rows in invalid_outputs:
        assert_prepared_fails_as_benchmark_row(
            replace(
                prepared,
                operation=lambda rows=rows: adapters.OperationOutcome(rows, len(rows)),
            ),
            workload,
            tmp_path,
        )


def test_benchmark_reports_failed_rows_for_invalid_entry_outputs(tmp_path: Path) -> None:
    class TitleList(list[str]):
        pass

    workload = fixtures.materialize_workload("tiny", tmp_path)
    prepared = adapters.RefkitAdapter().prepare("lookup_entries", workload, tmp_path)
    valid_rows = [{"key": record.key, "title": record.title} for record in workload.records]
    invalid_outputs = [
        [],
        [{**row, "key": "wrong"} for row in valid_rows],
        [{**row, "title": "wrong"} for row in valid_rows],
        [{**row, "title": []} for row in valid_rows],
        [{**row, "title": TitleList([str(row["title"])])} for row in valid_rows],
    ]

    for rows in invalid_outputs:
        assert_prepared_fails_as_benchmark_row(
            replace(
                prepared,
                operation=lambda rows=rows: adapters.OperationOutcome(rows, len(rows)),
            ),
            workload,
            tmp_path,
        )


def test_benchmark_reports_failed_rows_for_invalid_render_outputs(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    incomplete_bibliography = "\n".join(
        f"{record.family}, {record.given}. ({record.year})." for record in workload.records
    )
    scenarios = [
        replace(
            adapters.RefkitAdapter().prepare("access_rendered_html", workload, tmp_path),
            operation=lambda: adapters.OperationOutcome("no expected family", 1),
        ),
        replace(
            adapters.RefkitAdapter().prepare("access_rendered_tree", workload, tmp_path),
            operation=lambda: adapters.OperationOutcome([], 0),
        ),
        replace(
            adapters.RefkitAdapter().prepare("render_one_prepared_citation", workload, tmp_path),
            operation=lambda: adapters.OperationOutcome("(Wrong, 1999)", 1),
        ),
        replace(
            adapters.RefkitAdapter().prepare("render_prepared_bibliography", workload, tmp_path),
            operation=lambda: adapters.OperationOutcome("", 0),
        ),
        replace(
            adapters.RefkitAdapter().prepare("render_prepared_bibliography", workload, tmp_path),
            operation=lambda: adapters.OperationOutcome("", 3),
        ),
        replace(
            adapters.RefkitAdapter().prepare("render_prepared_bibliography", workload, tmp_path),
            operation=lambda: adapters.OperationOutcome(incomplete_bibliography, 3),
        ),
        replace(
            adapters.CiteprocPyAdapter().prepare("resolve_missing_reference", workload, tmp_path),
            operation=lambda: adapters.OperationOutcome("(missing-reference?)", 1, ""),
        ),
    ]

    for prepared in scenarios:
        assert_prepared_fails_as_benchmark_row(prepared, workload, tmp_path)


def test_benchmark_reports_failed_rows_for_invalid_raw_roundtrip_outputs(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    prepared = adapters.RefkitAdapter().prepare("roundtrip_raw_bibtex_edit", workload, tmp_path)
    missing_text = tmp_path / "missing-text.bib"
    missing_text.write_text("haystack", encoding="utf-8")
    missing_entry_count = tmp_path / "missing-entry-count.bib"
    missing_entry_count.write_text(
        "Edited Benchmark Title benchmark fixture with raw BibTeX blocks "
        "benchjournal Reference benchmark fixture item0001 item0002",
        encoding="utf-8",
    )

    for path in (missing_text, missing_entry_count):
        assert_prepared_fails_as_benchmark_row(
            replace(prepared, operation=lambda path=path: adapters.OperationOutcome(path, 3)),
            workload,
            tmp_path,
        )


def test_benchmark_reports_failed_rows_for_invalid_raw_block_outputs(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    prepared = adapters.RefkitAdapter().prepare("materialize_raw_blocks", workload, tmp_path)
    entry_rows = [{"kind": "entry", "key": key} for key in workload.keys]
    invalid_outputs = [
        ([{"kind": "comment", "key": ""}, *entry_rows], len(entry_rows)),
        ([{"kind": "comment", "key": ""}], 1),
        (entry_rows, len(entry_rows)),
        ([{"kind": "comment", "key": ""}, *entry_rows], len(entry_rows) + 1),
    ]

    for rows, count in invalid_outputs:
        assert_prepared_fails_as_benchmark_row(
            replace(
                prepared,
                operation=lambda rows=rows, count=count: adapters.OperationOutcome(rows, count),
            ),
            workload,
            tmp_path,
        )


def test_benchmark_reports_failed_rows_for_invalid_duplicate_outputs(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    prepared = adapters.RefkitAdapter().prepare("handle_duplicates", workload, tmp_path)
    valid_entry_signal = {
        "kind": "duplicate_entry",
        "key": "item0001",
        "field": "",
        "count": 2,
    }
    valid_field_signal = {
        "kind": "duplicate_field",
        "key": "item0002",
        "field": "title",
        "count": 2,
    }
    invalid_outputs = [
        ([valid_entry_signal, valid_field_signal], 1),
        [valid_entry_signal],
        [{**valid_entry_signal, "key": "wrong"}, valid_field_signal],
        [valid_entry_signal, {**valid_field_signal, "field": "wrong"}],
    ]

    for item in invalid_outputs:
        rows, count = item if isinstance(item, tuple) else (item, len(item))
        assert_prepared_fails_as_benchmark_row(
            replace(
                prepared,
                operation=lambda rows=rows, count=count: adapters.OperationOutcome(rows, count),
            ),
            workload,
            tmp_path,
        )


def test_each_lane_participant_has_correctness_check(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    adapters_by_name = {
        adapter.name: adapter
        for adapter in (
            adapters.RefkitAdapter(),
            adapters.CiteprocPyAdapter(),
            adapters.PolarsRefkitAdapter(),
            adapters.BibtexparserV2Adapter(),
            adapters.PybtexAdapter(),
        )
    }

    for lane in runner.LANES.values():
        for participant in lane.participants:
            prepared = adapters_by_name[participant.package].prepare(
                participant.adapter_operation, workload, tmp_path
            )
            outcome = run_prepared(prepared)
            assert outcome.count >= 1


def test_benchmark_render_metadata_describes_citation_requests(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    refkit_bibliography = adapters.RefkitAdapter().prepare(
        "render_prepared_bibliography", workload, tmp_path
    )
    refkit_path_bibliography = adapters.RefkitAdapter().prepare(
        "render_path_bibliography", workload, tmp_path
    )
    refkit_seen = adapters.RefkitAdapter().prepare("render_cited_bibliography", workload, tmp_path)
    citeproc_bibliography = adapters.CiteprocPyAdapter().prepare(
        "render_prepared_bibliography", workload, tmp_path
    )
    citeproc_seen = adapters.CiteprocPyAdapter().prepare(
        "render_cited_bibliography", workload, tmp_path
    )
    citeproc_missing = adapters.CiteprocPyAdapter().prepare(
        "resolve_missing_reference", workload, tmp_path
    )

    assert refkit_bibliography.metadata["citation_count"] == 0
    assert refkit_path_bibliography.metadata["citation_count"] == 0
    assert refkit_seen.metadata["citation_count"] == len(workload.records)
    assert citeproc_bibliography.metadata["citation_count"] == len(workload.records)
    assert citeproc_seen.metadata["citation_count"] == len(workload.records)
    with pytest.raises(AssertionError, match="expected count 1"):
        citeproc_missing.check(
            adapters.OperationOutcome("(missing-reference?)", 2, "missing-reference")
        )


def test_recovery_lane_schedules_only_recovery_adapters(tmp_path: Path) -> None:
    metadata = runner.machine_metadata("release")
    workload = fixtures.materialize_workload("tiny", tmp_path)

    result = runner.run_suite(
        lane_names=["input.dirty-bibtex"],
        input_sizes=["tiny"],
        rounds=2,
        warmups=0,
        build_mode="release",
    )

    rows = result["rows"]
    assert {row["package"] for row in rows} == {"refkit", "bibtexparser-2.x"}
    assert {row["status"] for row in rows} == {"ok"}
    assert {row["lane"] for row in rows} == {"input.dirty-bibtex"}
    assert all(row["capability"] == "normalized_bibliography_input" for row in rows)
    diagnostic_counts = {str(row["package"]): row["diagnostic_count"] for row in rows}
    assert diagnostic_counts["refkit"] == 4
    assert diagnostic_counts["bibtexparser-2.x"] == 2

    rows = runner.run_adapter_lane(
        adapter=adapters.BibtexparserV2Adapter(),
        lane=runner.LANES["input.dirty-bibtex"],
        participant=runner.participant(runner.BIBTEXPARSER_V2, "recover_dirty_bibtex"),
        workload=workload,
        directory=tmp_path,
        rounds=2,
        warmups=0,
        metadata=metadata,
    )

    assert len(rows) == 2
    for row in rows:
        assert row["status"] == "ok"
        assert row["phase"] == "input-recovery"
        assert row["operation_phase"] == "input-recovery"
        assert row["source_format"] == "dirty_bibtex"
        assert row["failed_block_count"] == 2
        assert row["diagnostic_count"] == 2
        assert row["operation_count"] == len(workload.records)
        assert "failed_blocks=2" in str(row["detail"])
        assert "ParsingFailedBlock:missing" in str(row["detail"])
        assert "DuplicateBlockKeyBlock:item0001" in str(row["detail"])


def test_diagnostics_raw_blocks_and_duplicate_lanes_record_source_metadata() -> None:
    result = runner.run_suite(
        lane_names=[
            "input.diagnostics",
            "raw-bibtex.blocks",
            "raw-bibtex.duplicates",
        ],
        input_sizes=["tiny"],
        rounds=1,
        warmups=0,
        build_mode="release",
    )
    rows = result["rows"]

    assert {row["status"] for row in rows} == {"ok"}
    assert {row["package"] for row in rows} == {"refkit", "bibtexparser-2.x"}
    source_formats = {str(row["lane"]): row["source_format"] for row in rows}
    assert source_formats["input.diagnostics"] == "dirty_bibtex"
    assert source_formats["raw-bibtex.blocks"] == "raw_bibtex"
    assert source_formats["raw-bibtex.duplicates"] == "duplicate_bibtex"
    duplicate_rows = [row for row in rows if row["lane"] == "raw-bibtex.duplicates"]
    assert {row["operation_count"] for row in duplicate_rows} == {2}
    assert all(str(row["source_name"]).endswith(":duplicate_bibtex") for row in duplicate_rows)


def test_recovery_parse_success_paths_cover_parser_return_values(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    recovery_path = tmp_path / "recoverable.bib"
    recovery_path.write_text(workload.bibtex, encoding="utf-8")
    recoverable = replace(
        workload,
        dirty_bibtex=workload.bibtex,
        dirty_bibtex_path=recovery_path,
    )

    for adapter in (
        adapters.RefkitAdapter(),
        adapters.BibtexparserV2Adapter(),
    ):
        prepared = adapter.prepare("recover_dirty_bibtex", recoverable, tmp_path)
        outcome = run_prepared(prepared)

        assert outcome.count == len(workload.records)


def test_repeated_citation_lane_uses_full_selected_input(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("medium", tmp_path)

    for adapter in (adapters.RefkitAdapter(), adapters.CiteprocPyAdapter()):
        prepared = adapter.prepare("render_repeated_citations", workload, tmp_path)
        outcome = prepared.operation()

        prepared.check(outcome)
        assert outcome.count == len(workload.records)
        assert prepared.metadata["citation_count"] == len(workload.records)


def test_polars_parse_includes_file_and_dataframe_setup(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    prepared = adapters.PolarsRefkitAdapter().prepare("parse_bibtex", workload, tmp_path)

    assert prepared.metadata["setup_included"] is True
    workload.bibtex_path.write_text("", encoding="utf-8")
    outcome = prepared.operation()

    assert outcome.count == 0


def test_polars_lookup_lane_uses_full_document_projection(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("medium", tmp_path)
    prepared = adapters.PolarsRefkitAdapter().prepare("lookup_entries_eager", workload, tmp_path)
    outcome = prepared.operation()

    prepared.check(outcome)
    assert prepared.metadata["source_format"] == "bibtex"
    assert outcome.count == 16


def test_polars_benchmark_lanes_record_execution_mode(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    adapter = adapters.PolarsRefkitAdapter()

    eager = adapter.prepare("project_fields_eager", workload, tmp_path)
    lazy = adapter.prepare("project_fields_lazy", workload, tmp_path)

    assert eager.metadata["execution_mode"] == "eager"
    assert lazy.metadata["execution_mode"] == "lazy"
    assert eager.metadata["setup_included"] is True
    assert lazy.metadata["setup_included"] is True
    run_prepared(lazy)


def test_polars_inspection_lanes_project_expected_rows(tmp_path: Path) -> None:
    workload = fixtures.materialize_workload("tiny", tmp_path)
    adapter = adapters.PolarsRefkitAdapter()
    prepared = [
        adapter.prepare("materialize_entry_rows_eager", workload, tmp_path),
        adapter.prepare("lookup_entries_eager", workload, tmp_path),
        adapter.prepare("project_fields_eager", workload, tmp_path),
    ]

    for item in prepared:
        outcome = item.operation()
        item.check(outcome)
        assert outcome.count >= 1


def test_run_adapter_lane_emits_unsupported_rows_for_missing_operation(tmp_path: Path) -> None:
    metadata = runner.machine_metadata("release")
    workload = fixtures.materialize_workload("tiny", tmp_path)
    rows = runner.run_adapter_lane(
        adapter=adapters.BibtexparserV2Adapter(),
        lane=runner.LANES["render.prepared-citation"],
        participant=runner.participant(runner.BIBTEXPARSER_V2, "render_one_prepared_citation"),
        workload=workload,
        directory=tmp_path,
        rounds=2,
        warmups=1,
        metadata=metadata,
    )

    assert len(rows) == 1
    assert rows[0]["status"] == "unsupported"
    assert rows[0]["phase"] == "setup"
    assert rows[0]["seconds"] == 0.0
    assert "benchmark operation" in str(rows[0]["detail"])


def test_real_arxiv_workload_records_citeproc_bibtex_path_limitation(
    tmp_path: Path,
) -> None:
    metadata = runner.machine_metadata("release")
    workload = fixtures.materialize_workload("arxiv", tmp_path)
    rows = runner.run_adapter_lane(
        adapter=adapters.CiteprocPyAdapter(),
        lane=runner.LANES["render.one-off-bibliography"],
        participant=runner.participant(runner.CITEPROC_PY, "render_path_bibliography"),
        workload=workload,
        directory=tmp_path,
        rounds=1,
        warmups=0,
        metadata=metadata,
    )

    assert len(rows) == 1
    assert rows[0]["status"] == "unsupported"
    assert rows[0]["input"] == "arxiv"
    assert rows[0]["workload_family"] == "arxiv_wild_subset"
    assert "non-entry bibliography rows" in str(rows[0]["detail"])


def test_run_adapter_lane_emits_failed_setup_rows(
    tmp_path: Path,
) -> None:
    class BrokenAdapter(adapters.PackageAdapter):
        name = "broken"
        distribution = "broken"

        def prepare_parse_bibtex(self, workload: object, directory: Path) -> object:
            raise RuntimeError("setup failed")

    metadata = runner.machine_metadata("release")
    workload = fixtures.materialize_workload("tiny", tmp_path)
    rows = runner.run_adapter_lane(
        adapter=BrokenAdapter(),
        lane=runner.LANES["input.bibtex"],
        participant=runner.participant("broken", "parse_bibtex"),
        workload=workload,
        directory=tmp_path,
        rounds=2,
        warmups=1,
        metadata=metadata,
    )

    assert rows[0]["status"] == "failed"
    assert rows[0]["phase"] == "setup"


def test_run_adapter_lane_emits_failed_execution_rows(
    tmp_path: Path,
) -> None:
    class FailingAdapter(adapters.PackageAdapter):
        name = "failing"
        distribution = "failing"

        def prepare_parse_bibtex(
            self,
            workload: object,
            directory: Path,
        ) -> adapters.PreparedOperation:
            return adapters.PreparedOperation(
                operation=lambda: adapters.OperationOutcome("wrong", 0),
                check=lambda outcome: (_ for _ in ()).throw(AssertionError("bad count")),
            )

    metadata = runner.machine_metadata("release")
    workload = fixtures.materialize_workload("tiny", tmp_path)
    rows = runner.run_adapter_lane(
        adapter=FailingAdapter(),
        lane=runner.LANES["input.bibtex"],
        participant=runner.participant("failing", "parse_bibtex"),
        workload=workload,
        directory=tmp_path,
        rounds=2,
        warmups=0,
        metadata=metadata,
    )

    assert len(rows) == 1
    assert rows[0]["status"] == "failed"
    assert rows[0]["phase"] == "input"


def test_run_adapter_lane_emits_failed_warmup_rows(
    tmp_path: Path,
) -> None:
    class WarmupFailingAdapter(adapters.PackageAdapter):
        name = "warmup-failing"
        distribution = "warmup-failing"

        def prepare_parse_bibtex(
            self,
            workload: object,
            directory: Path,
        ) -> adapters.PreparedOperation:
            return adapters.PreparedOperation(
                operation=lambda: adapters.OperationOutcome("wrong", 0),
                check=lambda outcome: (_ for _ in ()).throw(AssertionError("warmup failed")),
            )

    metadata = runner.machine_metadata("release")
    rows = runner.run_adapter_lane(
        adapter=WarmupFailingAdapter(),
        lane=runner.LANES["input.bibtex"],
        participant=runner.participant("warmup-failing", "parse_bibtex"),
        workload=fixtures.materialize_workload("tiny", tmp_path),
        directory=tmp_path,
        rounds=2,
        warmups=1,
        metadata=metadata,
    )

    assert len(rows) == 1
    assert rows[0]["phase"] == "input"
    assert rows[0]["operation_phase"] == "input"
    assert rows[0]["round"] == 0
    assert rows[0]["seconds"] == 0.0
    assert rows[0]["status"] == "failed"
    assert rows[0]["detail"] == "AssertionError('warmup failed')"
    assert isinstance(rows[0]["setup_seconds"], float)
    assert rows[0]["setup_seconds"] >= 0.0
    assert rows[0]["source_format"] == "unknown"


def test_run_adapter_lane_reports_cleanup_failure(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    class CleanupFailingAdapter(adapters.PackageAdapter):
        name = "cleanup-failing"
        distribution = "cleanup-failing"

        def prepare_parse_bibtex(
            self,
            workload: object,
            directory: Path,
        ) -> adapters.PreparedOperation:
            def check(outcome: adapters.OperationOutcome) -> None:
                assert outcome.count == 1

            return adapters.PreparedOperation(
                operation=lambda: adapters.OperationOutcome("ok", 1),
                check=check,
                cleanup=lambda: (_ for _ in ()).throw(RuntimeError("cleanup failed")),
            )

    metadata = runner.machine_metadata("release")
    rows = runner.run_adapter_lane(
        adapter=CleanupFailingAdapter(),
        lane=runner.LANES["input.bibtex"],
        participant=runner.participant("cleanup-failing", "parse_bibtex"),
        workload=fixtures.materialize_workload("tiny", tmp_path),
        directory=tmp_path,
        rounds=1,
        warmups=0,
        metadata=metadata,
    )

    assert rows[0]["status"] == "ok"
    assert "cleanup failed" in capsys.readouterr().err


def test_run_suite_writes_only_scheduled_lane_rows() -> None:
    result = runner.run_suite(
        lane_names=["input.bibtex", "render.prepared-citation"],
        input_sizes=["tiny"],
        rounds=1,
        warmups=1,
        build_mode="release",
    )
    rows = result["rows"]

    assert isinstance(result["metadata"], dict)
    assert any(row["status"] == "ok" for row in rows)
    assert {row["status"] for row in rows} <= {"ok", "failed"}
    assert {row["package"] for row in rows if row["lane"] == "input.bibtex"} == {
        "refkit",
        "polars-refkit",
        "bibtexparser-2.x",
        "pybtex",
    }
    assert sorted(
        row_string(row, "execution_mode")
        for row in rows
        if row["lane"] == "input.bibtex" and row["package"] == "polars-refkit"
    ) == ["eager", "lazy"]
    assert {row["package"] for row in rows if row["lane"] == "render.prepared-citation"} == {
        "refkit",
        "citeproc-py",
    }
    assert all(row["input"] == "tiny" for row in rows)
    expected_lanes = {
        "input.bibtex": ("normalized_bibliography_input", "bibtex_input"),
        "render.prepared-citation": ("citation_rendering", "prepared_citation"),
    }
    for row in rows:
        assert {"lane", "package", "status", "seconds", "capability", "workflow"} <= set(row)
        assert row["lane"] in expected_lanes
        assert (row["capability"], row["workflow"]) == expected_lanes[str(row["lane"])]
        assert row["input_size"] == "tiny"
        assert row["workload_family"] == "synthetic_scale"
        assert str(row["source_name"]).startswith("synthetic_scale:tiny:")
        assert row["source_license"] == "Apache-2.0"
        assert row["record_count"] == 3
        assert row["failed_block_count"] == 0
        assert row["diagnostic_count"] == 0
        assert row["rounds"] == 1
        assert row["warmups"] == 1
        assert isinstance(row["setup_seconds"], float)
        assert row["setup_seconds"] >= 0.0
        assert row["adapter_version"] == row["package_version"]
        assert row["execution_mode"] in {"", "eager", "lazy"}
        if row["package"] == "polars-refkit" and row["status"] == "ok":
            assert row["execution_mode"] in {"eager", "lazy"}


def test_run_suite_exercises_real_arxiv_workload() -> None:
    result = runner.run_suite(
        lane_names=[
            "input.bibtex-text",
            "inspect.fields",
            "render.prepared-citation",
            "bulk.polars.bibliography",
        ],
        input_sizes=["arxiv"],
        rounds=1,
        warmups=0,
        build_mode="release",
    )
    rows = result["rows"]

    assert rows
    assert {row["status"] for row in rows} == {"ok"}
    assert {row["input"] for row in rows} == {"arxiv"}
    assert {row["workload_family"] for row in rows} == {"arxiv_wild_subset"}
    assert {row["record_count"] for row in rows} == {12}
    assert all(row["source_license"] == "mixed-arxiv-source-licenses" for row in rows)
    assert any(row["package"] == "citeproc-py" for row in rows)
    assert any(row["package"] == "polars-refkit" for row in rows)


def test_run_suite_can_use_filtered_adapters() -> None:
    result = runner.run_suite(
        lane_names=["input.bibtex", "render.prepared-citation"],
        input_sizes=["tiny"],
        rounds=1,
        warmups=0,
        build_mode="release",
        package_adapters=[adapters.RefkitAdapter()],
    )

    rows = result["rows"]
    assert rows
    assert {row["package"] for row in rows} == {"refkit"}
    assert {row["lane"] for row in rows} == {"input.bibtex", "render.prepared-citation"}


def test_write_json_and_csv_outputs(tmp_path: Path) -> None:
    result = runner.run_suite(
        lane_names=["input.bibtex"],
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
    assert csv_rows[0]["lane"] == "input.bibtex"
    assert csv_rows[0]["capability"] == "normalized_bibliography_input"
    assert csv_rows[0]["workflow"] == "bibtex_input"
    assert csv_rows[0]["workload_family"] == "synthetic_scale"
    assert csv_rows[0]["source_name"] == "synthetic_scale:tiny:bibtex"
    assert csv_rows[0]["source_license"] == "Apache-2.0"
    assert csv_rows[0]["record_count"] == "3"
    assert csv_rows[0]["source_format"] == "bibtex"
    assert csv_rows[0]["failed_block_count"] == "0"
    assert csv_rows[0]["diagnostic_count"] == "0"
    assert csv_rows[0]["input_sha256"]


def test_main_runs_lane_and_writes_outputs(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    json_path = tmp_path / "result.json"
    csv_path = tmp_path / "result.csv"

    exit_code = runner.main(
        [
            "--lane",
            "input.bibtex",
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
    assert '"rows": 5' in capsys.readouterr().out


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
                    "lane": "input.bibtex",
                    "group": "input.normalized",
                    "capability": "normalized_bibliography_input",
                    "workflow": "bibtex_input",
                    "package": "fake",
                    "package_version": "0",
                    "phase": "input",
                    "input": "tiny",
                    "round": 1,
                    "seconds": 0.0,
                    "detail": "failure",
                    "python": "3",
                    "os": "test",
                    "cpu": "test",
                    "refkit_version": "0.0.1",
                    "refkit_commit": "test",
                    "build_mode": "release",
                }
            ],
        }

    monkeypatch.setattr(runner, "run_suite", fake_run_suite)

    assert runner.main(["--lane", "input.bibtex", "--input", "tiny"]) == 1
    assert '"failed": 1' in capsys.readouterr().out
