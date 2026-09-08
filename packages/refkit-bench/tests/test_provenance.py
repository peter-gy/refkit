from __future__ import annotations


def test_measured_wrapper_changes_have_distinct_identity(tmp_path, monkeypatch):
    from types import SimpleNamespace

    from refkit_bench import provenance

    package = tmp_path / "refkit"
    package.mkdir()
    native = package / "_native.so"
    native.write_bytes(b"native-library")
    wrapper = package / "__init__.py"
    wrapper.write_text("value = 1\n")
    installed = SimpleNamespace(version="1.0", read_text=lambda name: None)
    module = SimpleNamespace(__file__=str(native), build_mode="release")
    monkeypatch.setattr(provenance, "distribution", lambda name: installed)
    monkeypatch.setattr(provenance.importlib, "import_module", lambda name: module)
    before = provenance.artifact("refkit")
    wrapper.write_text("value = 2\n")
    after = provenance.artifact("refkit")
    assert before["artifact_sha256"] != after["artifact_sha256"]
    assert after["build_mode"] == "release"
    assert after["artifact_source_revision"] == "unknown"


def test_installed_harness_does_not_inherit_an_ambient_checkout(tmp_path, monkeypatch):
    from refkit_bench import provenance

    installed = tmp_path / "environment/site-packages/refkit_bench"
    installed.mkdir(parents=True)
    monkeypatch.setattr(provenance, "SOURCE", installed)
    details = provenance.environment()
    assert details["runner_commit"] == "unknown"
    assert details["runner_dirty"] is None


def test_harness_records_unknown_revision_when_git_is_unavailable(monkeypatch):
    from refkit_bench import provenance

    monkeypatch.setattr(provenance.shutil, "which", lambda name: None)
    details = provenance.environment()
    assert details["runner_commit"] == "unknown"
    assert details["runner_dirty"] is None
