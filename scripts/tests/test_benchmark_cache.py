from __future__ import annotations

import io
import json
import subprocess
import zipfile
from hashlib import sha256
from types import SimpleNamespace

import pytest

from scripts import benchmark_cache, benchmark_report
from scripts.benchmark_inputs import fingerprint
from scripts.benchmark_release import package
from scripts.tests.test_benchmark_ci import CANDIDATE, report

IDENTITY = "e" * 64


def evidence():
    data = {**report(), "inputs_sha256": IDENTITY}
    files = {"report.json": json.dumps(data).encode()}
    for name in ("Linux", "Windows", "macOS"):
        for revision in ("baseline", "candidate"):
            prefix = f"{name}/{revision}"
            timings = b'{"benchmarks": []}'
            files[f"{prefix}/timings.json"] = timings
            files[f"{prefix}/manifest.json"] = json.dumps(
                {
                    "status": "complete",
                    "profile": "measurement",
                    "timings_sha256": sha256(timings).hexdigest(),
                    "checks": [{"name": "parse.bibtex/real/refkit", "status": "ok"}],
                }
            ).encode()
    return files


def write_evidence(directory):
    for name, content in evidence().items():
        target = directory / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content)


def git(root, *args):
    return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()


@pytest.fixture
def source(tmp_path):
    git(tmp_path, "init", "-q")
    git(tmp_path, "config", "user.name", "Benchmark Test")
    git(tmp_path, "config", "user.email", "benchmark@example.test")
    (tmp_path / "Cargo.toml").write_text("[workspace]\n")
    git(tmp_path, "add", ".")
    git(tmp_path, "commit", "-qm", "Initial input")
    return tmp_path


def test_documentation_and_test_changes_reuse_runtime_fingerprint(source):
    before = fingerprint(source, "HEAD")
    for name in (
        "README.md",
        "docs/guide.md",
        "scripts/AGENTS.md",
        "packages/refkit/tests/test_api.py",
    ):
        target = source / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text("new content")
    git(source, "add", ".")
    git(source, "commit", "-qm", "Update documentation and tests")
    assert fingerprint(source, "HEAD") == before


@pytest.mark.parametrize(
    "name",
    [
        "crates/refkit-core/src/style.rs",
        "packages/refkit/src/refkit/types.py",
        "packages/polars-refkit/rust/Cargo.lock",
        "uv.lock",
        "packages/refkit-bench/src/refkit_bench/data/styles/author-date.csl",
        ".github/actions/configure-rust-build/action.yml",
    ],
)
def test_runtime_dependency_fixture_and_build_changes_invalidate_evidence(source, name):
    before = fingerprint(source, "HEAD")
    target = source / name
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text("new input")
    git(source, "add", ".")
    git(source, "commit", "-qm", "Change benchmark input")
    assert fingerprint(source, "HEAD") != before


def test_moving_a_runtime_file_changes_its_fingerprint(source):
    target = source / "packages/refkit/src/refkit"
    target.mkdir(parents=True)
    (target / "first.py").write_text("value = 1\n")
    git(source, "add", ".")
    git(source, "commit", "-qm", "Add runtime module")
    before = fingerprint(source, "HEAD")
    (target / "first.py").rename(target / "second.py")
    git(source, "add", "-A")
    git(source, "commit", "-qm", "Move runtime module")
    assert fingerprint(source, "HEAD") != before


@pytest.fixture
def cache(monkeypatch):
    state = SimpleNamespace(
        files=evidence(),
        artifact={
            "id": 5,
            "name": f"benchmark-results-{IDENTITY}",
            "expired": False,
            "size_in_bytes": 3000,
            "workflow_run": {"id": 10},
        },
        run={
            "event": "push",
            "head_branch": "main",
            "path": ".github/workflows/benchmarks.yml",
            "head_repository": {"full_name": "owner/repo"},
        },
    )

    def api(route):
        return {"artifacts": [state.artifact]} if "/artifacts?" in route else state.run

    def download(*args, **kwargs):
        archive = io.BytesIO()
        with zipfile.ZipFile(archive, "w") as bundle:
            for name, contents in state.files.items():
                bundle.writestr(name, contents)
        return SimpleNamespace(stdout=archive.getvalue())

    monkeypatch.setattr(benchmark_cache, "api", api)
    monkeypatch.setattr(benchmark_cache.subprocess, "run", download)
    return state


def test_complete_matching_results_are_reused(cache):
    assert benchmark_cache.find("owner/repo", IDENTITY) == "10"


@pytest.mark.parametrize(
    "change", ["expired", "pull_request", "branch", "fingerprint", "raw_hash", "missing_raw"]
)
def test_untrusted_incomplete_or_mismatched_results_require_measurement(cache, change):
    if change == "expired":
        cache.artifact["expired"] = True
    elif change == "pull_request":
        cache.run["event"] = "pull_request"
    elif change == "branch":
        cache.run["head_branch"] = "feature"
    elif change == "fingerprint":
        data = json.loads(cache.files["report.json"])
        data["inputs_sha256"] = "f" * 64
        cache.files["report.json"] = json.dumps(data).encode()
    elif change == "raw_hash":
        cache.files["Linux/candidate/timings.json"] = b"changed"
    else:
        del cache.files["Windows/baseline/manifest.json"]
    assert benchmark_cache.find("owner/repo", IDENTITY) == ""


def test_release_results_can_supply_the_main_branch_cache(cache):
    cache.run.update(path=".github/workflows/publish.yml", head_branch="v0.0.7")
    assert benchmark_cache.find("owner/repo", IDENTITY) == "10"


def test_reused_report_preserves_measurements_and_identifies_current_commit(tmp_path, monkeypatch):
    write_evidence(tmp_path)
    monkeypatch.setenv("REUSED_RUN", "10")
    monkeypatch.setenv("INPUTS_SHA256", IDENTITY)
    monkeypatch.setenv("BASELINE_INPUTS_SHA256", IDENTITY)
    monkeypatch.setattr(
        "sys.argv", ["report", str(tmp_path), "--baseline", CANDIDATE, "--candidate", "d" * 40]
    )
    assert benchmark_report.main() == 0
    data = json.loads((tmp_path / "report.json").read_text())
    assert data["candidate_sha"] == CANDIDATE
    assert data["request"]["candidate_sha"] == "d" * 40
    text = (tmp_path / "summary.md").read_text()
    assert "Performance: unchanged inputs" in text
    assert "1.6" in text
    assert "-20.0%" not in text


def test_release_assets_include_json_markdown_and_raw_samples(tmp_path):
    directory = tmp_path / "evidence"
    write_evidence(directory)
    output = tmp_path / "assets"
    package(directory, output, IDENTITY, CANDIDATE)
    assert json.loads((output / "benchmark-results.json").read_text())["inputs_sha256"] == IDENTITY
    assert "| macOS |" in (output / "benchmark-results.md").read_text()
    with zipfile.ZipFile(output / "benchmark-results.zip") as bundle:
        assert bundle.read("Windows/candidate/timings.json") == b'{"benchmarks": []}'


def test_release_rejects_evidence_from_different_runtime_inputs(tmp_path):
    write_evidence(tmp_path)
    with pytest.raises(ValueError, match="release inputs"):
        package(tmp_path, tmp_path / "assets", "f" * 64, CANDIDATE)


def test_unchanged_main_inputs_skip_measurement_when_artifacts_expire(tmp_path, monkeypatch):
    monkeypatch.setenv("UNCHANGED_ONLY", "true")
    monkeypatch.setenv("INPUTS_SHA256", IDENTITY)
    monkeypatch.setenv("BASELINE_INPUTS_SHA256", IDENTITY)
    monkeypatch.setattr(
        "sys.argv", ["report", str(tmp_path), "--baseline", CANDIDATE, "--candidate", "d" * 40]
    )
    assert benchmark_report.main() == 0
    data = json.loads((tmp_path / "report.json").read_text())
    assert all(item["status"] == "unmeasured" for item in data["platforms"].values())
    assert "Performance: unchanged inputs" in (tmp_path / "summary.md").read_text()
    with pytest.raises(ValueError, match="all three platforms"):
        package(tmp_path, tmp_path / "release-assets", IDENTITY, "d" * 40)


def test_changed_main_transition_requires_matching_baseline_but_release_can_reuse(cache):
    assert benchmark_cache.find("owner/repo", IDENTITY, "a" * 64) == ""
    assert benchmark_cache.find("owner/repo", IDENTITY, "a" * 64, candidate_only=True) == "10"
    data = json.loads(cache.files["report.json"])
    data["baseline_inputs_sha256"] = "a" * 64
    cache.files["report.json"] = json.dumps(data).encode()
    assert benchmark_cache.find("owner/repo", IDENTITY, "a" * 64) == "10"


def test_matching_transition_reuses_its_original_comparison(tmp_path, monkeypatch):
    write_evidence(tmp_path)
    data = json.loads((tmp_path / "report.json").read_text())
    data["baseline_inputs_sha256"] = "a" * 64
    (tmp_path / "report.json").write_text(json.dumps(data))
    monkeypatch.setenv("REUSED_RUN", "10")
    monkeypatch.setenv("INPUTS_SHA256", IDENTITY)
    monkeypatch.setenv("BASELINE_INPUTS_SHA256", "a" * 64)
    monkeypatch.setattr(
        "sys.argv", ["report", str(tmp_path), "--baseline", CANDIDATE, "--candidate", "d" * 40]
    )
    assert benchmark_report.main() == 0
    text = (tmp_path / "summary.md").read_text()
    assert "reuses this measured comparison" in text
    assert "-20.0%" in text
