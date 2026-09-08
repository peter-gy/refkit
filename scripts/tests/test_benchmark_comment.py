from __future__ import annotations

import io
import json
import zipfile
from types import SimpleNamespace

import pytest

from scripts import benchmark_comment
from scripts.tests.test_benchmark_cache import evidence
from scripts.tests.test_benchmark_ci import CANDIDATE, report


@pytest.fixture
def github(monkeypatch):
    run = {
        "event": "push",
        "path": ".github/workflows/benchmarks.yml",
        "id": 100,
        "run_attempt": 1,
        "head_sha": CANDIDATE,
        "head_branch": "main",
        "head_repository": {"full_name": "owner/repo"},
    }
    writes = []
    comments = []
    runs = [run]

    def api(route, *, method="GET", body=None):
        if method != "GET":
            writes.append((route, method, body))
            return {}
        if "/comments?" in route:
            return comments
        if "/runs?" in route:
            return {"workflow_runs": runs}
        raise AssertionError(route)

    monkeypatch.setattr(benchmark_comment, "api", api)
    monkeypatch.setattr(benchmark_comment, "read_report", lambda *_: report())
    return SimpleNamespace(run=run, writes=writes, comments=comments, runs=runs)


def test_main_run_posts_numeric_report_on_measured_commit(github):
    benchmark_comment.publish("owner/repo", github.run)
    route, method, payload = github.writes[0]
    assert route == f"repos/owner/repo/commits/{CANDIDATE}/comments"
    assert method == "POST"
    assert "-20.0%" in payload["body"]
    assert "actions/runs/100/attempts/1" in payload["body"]


def test_publisher_updates_its_existing_commit_comment(github):
    github.comments.extend(
        [
            {"id": 2, "user": {"login": "someone"}, "body": benchmark_comment.MARKER},
            {"id": 3, "user": {"login": "github-actions[bot]"}, "body": benchmark_comment.MARKER},
        ]
    )
    benchmark_comment.publish("owner/repo", github.run)
    assert github.writes[0][:2] == ("repos/owner/repo/comments/3", "PATCH")


@pytest.mark.parametrize("change", ["event", "branch", "repository", "workflow", "newer", "rerun"])
def test_stale_or_unrelated_run_cannot_update_comment(github, change):
    if change == "event":
        github.run["event"] = "pull_request"
    elif change == "branch":
        github.run["head_branch"] = "feature"
    elif change == "repository":
        github.run["head_repository"]["full_name"] = "other/repo"
    elif change == "workflow":
        github.run["path"] = ".github/workflows/ci.yml"
    else:
        github.runs.append(
            {**github.run, **({"id": 101} if change == "newer" else {"run_attempt": 2})}
        )
    benchmark_comment.publish("owner/repo", github.run)
    assert github.writes == []


def test_missing_artifact_posts_incomplete_commit_status(github, monkeypatch):
    def missing(*_):
        raise ValueError("artifact unavailable")

    monkeypatch.setattr(benchmark_comment, "read_report", missing)
    benchmark_comment.publish("owner/repo", github.run)
    assert "Measurements are incomplete" in github.writes[0][2]["body"]


def test_report_for_another_commit_cannot_publish_performance_claims(github, monkeypatch):
    data = report()
    data["candidate_sha"] = "d" * 40
    monkeypatch.setattr(benchmark_comment, "read_report", lambda *_: data)
    benchmark_comment.publish("owner/repo", github.run)
    body = github.writes[0][2]["body"]
    assert "Measurements are incomplete" in body
    assert "-20.0%" not in body


def test_archive_is_read_as_data_without_extracting_files(monkeypatch):
    archive = io.BytesIO()
    with zipfile.ZipFile(archive, "w") as bundle:
        for name, content in evidence().items():
            bundle.writestr(name, content)
        bundle.writestr("../../executable.py", "raise Exception('untrusted')")
    monkeypatch.setattr(
        benchmark_comment,
        "api",
        lambda *_: {
            "artifacts": [
                {"id": 5, "name": "benchmark-results-1", "expired": False, "size_in_bytes": 3000},
            ]
        },
    )
    monkeypatch.setattr(
        benchmark_comment.subprocess,
        "run",
        lambda *args, **kwargs: SimpleNamespace(stdout=archive.getvalue()),
    )
    assert benchmark_comment.read_report("owner/repo", {"id": 100, "run_attempt": 1}) == json.loads(
        evidence()["report.json"]
    )
