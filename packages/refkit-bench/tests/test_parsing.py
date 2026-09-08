from __future__ import annotations

import pytest

from refkit_bench.fixtures import load_workload
from refkit_bench.parsing import prepare_parse


@pytest.mark.parametrize("package", ["refkit", "bibtexparser", "pybtex"])
def test_synthetic_bibliography_parses_into_complete_records(package: str) -> None:
    prepared = prepare_parse("parse.bibtex", load_workload("tiny"), package)
    prepared.check(prepared.operation())


@pytest.mark.parametrize("package", ["refkit", "bibtexparser", "pybtex"])
def test_lookup_materializes_first_middle_last_titles(package: str) -> None:
    prepared = prepare_parse("inspect.lookup", load_workload("medium"), package)
    assert prepared.operation() == [
        {"key": "item0001", "title": "Reference Work 0001"},
        {"key": "item0025", "title": "Reference Work 0025"},
        {"key": "item0048", "title": "Reference Work 0048"},
    ]


@pytest.mark.parametrize("package", ["refkit", "bibtexparser", "pybtex"])
@pytest.mark.parametrize(
    ("original", "changed"),
    [
        ("item0001", "wrong-key"),
        ("@article", "@book"),
        ("Reference Work 0001", "Other title"),
        ("Family0001, Given0001", "Other, Author"),
        ("year = {2001}", "year = {1999}"),
        ("Journal of Citation Benchmarks", "Other Journal"),
        ("volume = {2}", "volume = {99}"),
        ("pages = {3-11}", "pages = {3-12}"),
        ("10.5555/refkit.bench.0001", "10.1234/other"),
    ],
)
def test_parse_check_rejects_changes_to_each_compared_field(
    package: str, original: str, changed: str
) -> None:
    from dataclasses import replace

    workload = load_workload("tiny")
    prepared = prepare_parse("parse.bibtex", workload, package)
    changed_workload = replace(workload, bibtex=workload.bibtex.replace(original, changed, 1))
    changed_model = prepare_parse("parse.bibtex", changed_workload, package).operation()
    with pytest.raises(AssertionError):
        prepared.check(changed_model)


@pytest.mark.parametrize("package", ["refkit", "bibtexparser"])
def test_raw_edit_preserves_fields_and_source_blocks_across_calls(package: str) -> None:
    prepared = prepare_parse("raw.edit", load_workload("real"), package)
    first = prepared.operation()
    prepared.check(first)
    second = prepared.operation()
    prepared.check(second)
    assert second == first


@pytest.mark.parametrize(
    ("original", "changed"),
    [
        ("% benchmark raw comment", ""),
        ("Journal of Citation Benchmarks", "Changed journal"),
        ("Benchmark preamble", "Changed preamble"),
        ("10.5555/refkit.bench.0002", "10.1234/wrong"),
    ],
)
def test_raw_edit_check_rejects_unrelated_data_loss(original: str, changed: str) -> None:
    prepared = prepare_parse("raw.edit", load_workload("tiny"), "refkit")
    result = prepared.operation()
    assert isinstance(result, str)
    with pytest.raises(AssertionError, match="output mismatch"):
        prepared.check(result.replace(original, changed, 1))
