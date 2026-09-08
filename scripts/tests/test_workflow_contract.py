from pathlib import Path

import yaml


def test_documentation_deployment_waits_for_required_checks() -> None:
    root = Path(__file__).resolve().parents[2]
    workflow = yaml.safe_load((root / ".github/workflows/ci.yml").read_text())
    jobs = workflow["jobs"]

    assert jobs["deploy-docs"]["needs"] == "check"
    assert {"source-checks", "docs"} <= set(jobs["check"]["needs"])


def test_benchmarks_measure_platforms_independently_and_preserve_successful_reruns() -> None:
    root = Path(__file__).resolve().parents[2]
    workflow = yaml.safe_load((root / ".github/workflows/benchmarks.yml").read_text())
    assert set(workflow["on"]) == {"push", "workflow_call", "workflow_dispatch"}
    assert workflow["on"]["push"]["branches"] == ["main"]
    assert workflow["jobs"]["results"]["uses"] == "./.github/workflows/benchmark-results.yml"
    workflow = yaml.safe_load((root / ".github/workflows/benchmark-results.yml").read_text())
    assert workflow["concurrency"]["queue"] == "max"
    measure = workflow["jobs"]["measure"]
    assert set(measure["strategy"]["matrix"]["os"]) == {
        "ubuntu-latest",
        "windows-latest",
        "macos-15",
    }
    assert measure["strategy"]["fail-fast"] is False
    assert measure["needs"] == "lookup"
    assert "needs.lookup.outputs.run-id == ''" in measure["if"]
    assert "inputs.fingerprint != inputs.baseline-fingerprint" in measure["if"]
    assert "inputs.require-measurements" in measure["if"]
    upload = next(step for step in measure["steps"] if "upload-artifact@" in step.get("uses", ""))
    assert upload["with"]["overwrite"] is True
    assert "run_attempt" not in upload["with"]["name"]
    report = workflow["jobs"]["report"]
    assert set(report["needs"]) == {"lookup", "measure"}
    assert "always()" in report["if"]
    assert workflow["permissions"] == {"contents": "read", "actions": "read"}


def test_comment_publisher_executes_default_branch_code_with_scoped_permissions() -> None:
    root = Path(__file__).resolve().parents[2]
    workflow = yaml.safe_load((root / ".github/workflows/benchmark-comment.yml").read_text())
    assert workflow["on"]["workflow_run"] == {
        "workflows": ["Benchmarks"],
        "types": ["completed"],
    }
    job = workflow["jobs"]["comment"]
    assert job["permissions"] == {"contents": "write", "actions": "read"}
    checkout = job["steps"][0]
    assert checkout["with"]["ref"] == "${{ github.event.repository.default_branch }}"
    assert checkout["with"]["persist-credentials"] is False


def test_release_uses_shared_benchmarks_and_attaches_matching_evidence() -> None:
    root = Path(__file__).resolve().parents[2]
    workflow = yaml.safe_load((root / ".github/workflows/publish.yml").read_text())
    jobs = workflow["jobs"]
    assert jobs["benchmarks"]["uses"] == "./.github/workflows/benchmarks.yml"
    assert "benchmarks" in jobs["release-complete"]["needs"]
    steps = jobs["release-notes"]["steps"]
    download = next(step for step in steps if step.get("name") == "Download benchmark evidence")
    assert download["with"]["name"] == "${{ needs.benchmarks.outputs.artifact-name }}"
    attach = next(step for step in steps if step.get("name") == "Attach benchmark results")
    assert "python -m scripts.benchmark_release benchmark-evidence" in attach["run"]
