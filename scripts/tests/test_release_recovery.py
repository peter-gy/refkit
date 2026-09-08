from __future__ import annotations

import pytest

from scripts.release_recovery import validate


@pytest.fixture
def release():
    run = {
        "event": "push",
        "path": ".github/workflows/publish.yml",
        "head_branch": "v0.2.0",
        "head_sha": "abc123",
        "status": "completed",
    }
    jobs = [
        {"name": name, "conclusion": "success"}
        for name in [
            "refkit / Publish to PyPI",
            "polars-refkit / Publish to PyPI",
            "refkit-js artifacts / Build npm package",
            "refkit-js artifacts / Browser package",
            "Benchmark evidence / results / Consolidated results",
        ]
    ]
    jobs.extend(
        {"name": f"refkit-js artifacts / Node {node} / {runner}", "conclusion": "success"}
        for node in ("22.19.0", "24", "26")
        for runner in ("ubuntu-latest", "macos-latest", "windows-latest")
    )
    artifacts = [
        {"name": "refkit-js-npm", "expired": False},
        {"name": "benchmark-results-" + "a" * 64, "expired": False},
    ]
    return run, jobs, artifacts


def test_retained_release_uses_successful_tag_artifacts(release):
    assert (
        validate(release[0], release[1], release[2], "v0.2.0", "abc123")
        == "benchmark-results-" + "a" * 64
    )


@pytest.mark.parametrize(
    ("field", "value"),
    [
        ("head_sha", "other"),
        ("head_branch", "v0.1.0"),
        ("event", "pull_request"),
        ("status", "in_progress"),
    ],
)
def test_retained_release_requires_the_completed_tag_run(release, field, value):
    release[0][field] = value
    with pytest.raises(ValueError, match="requested tag commit"):
        validate(release[0], release[1], release[2], "v0.2.0", "abc123")


@pytest.mark.parametrize(
    "name",
    [
        "polars-refkit / Publish to PyPI",
        "refkit-js artifacts / Node 22.19.0 / windows-latest",
        "Benchmark evidence / results / Consolidated results",
    ],
)
def test_retained_release_requires_publication_and_runtime_gates(release, name):
    next(job for job in release[1] if job["name"] == name)["conclusion"] = "failure"
    with pytest.raises(ValueError, match="source release gates"):
        validate(release[0], release[1], release[2], "v0.2.0", "abc123")


def test_retained_release_rejects_expired_artifacts(release):
    release[2][0]["expired"] = True
    with pytest.raises(ValueError, match="retain one npm artifact"):
        validate(release[0], release[1], release[2], "v0.2.0", "abc123")


def test_retained_release_requires_every_node_platform(release):
    release[1].remove(
        next(
            job
            for job in release[1]
            if job["name"] == "refkit-js artifacts / Node 26 / macos-latest"
        )
    )
    with pytest.raises(ValueError, match="Node 26 / macos-latest"):
        validate(release[0], release[1], release[2], "v0.2.0", "abc123")


def test_retained_release_uses_latest_attempt_for_each_job(release):
    name = "refkit-js artifacts / Node 24 / ubuntu-latest"
    release[1].insert(0, {"name": name, "conclusion": "failure", "run_attempt": 2})
    with pytest.raises(ValueError, match="Node 24 / ubuntu-latest"):
        validate(release[0], release[1], release[2], "v0.2.0", "abc123")
    release[1][0]["conclusion"] = "success"
    assert (
        validate(release[0], release[1], release[2], "v0.2.0", "abc123")
        == "benchmark-results-" + "a" * 64
    )
