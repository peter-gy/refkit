from __future__ import annotations

import base64
import csv
import hashlib
import io
import json
import zipfile
from pathlib import Path

import agent_plugins
import pytest

import scripts.augment_agent_plugin as agent_plugin_artifact
from scripts.augment_agent_plugin import augment_wheel, main


def _project(root: Path) -> Path:
    project = root / "package"
    project.mkdir()
    (project / "pyproject.toml").write_text(
        '[tool.agent-plugins]\nroot = ".."\n',
        encoding="utf-8",
    )
    (root / "plugin.json").write_text('{"name": "example"}\n', encoding="utf-8")
    skill = root / "skills" / "example"
    skill.mkdir(parents=True)
    (skill / "SKILL.md").write_text(
        "---\nname: example\ndescription: Example skill.\n---\n\n# Example\n",
        encoding="utf-8",
    )
    return project


def _wheel(path: Path) -> None:
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr("example/__init__.py", "")
        archive.writestr("example-1.0.0.dist-info/WHEEL", "Wheel-Version: 1.0\n")
        archive.writestr("example-1.0.0.dist-info/RECORD", "")


def test_augment_wheel_adds_plugin_payload_and_marker(tmp_path: Path) -> None:
    project = _project(tmp_path)
    wheel = tmp_path / "example-1.0.0-py3-none-any.whl"
    _wheel(wheel)

    augment_wheel(wheel, project=project)

    with zipfile.ZipFile(wheel) as archive:
        names = set(archive.namelist())
        marker_path = "example-1.0.0.dist-info/agent_plugins.json"
        marker = json.loads(archive.read(marker_path))
        records = {
            name: (digest, size)
            for name, digest, size in csv.reader(
                io.StringIO(archive.read("example-1.0.0.dist-info/RECORD").decode())
            )
        }
        assert marker == {
            "root": "example-1.0.0.agent-plugin",
            "files": ["plugin.json", "skills/example/SKILL.md"],
        }
        assert {
            "example/__init__.py",
            "example-1.0.0.agent-plugin/plugin.json",
            "example-1.0.0.agent-plugin/skills/example/SKILL.md",
        } <= names
        for name in (
            marker_path,
            "example-1.0.0.agent-plugin/plugin.json",
            "example-1.0.0.agent-plugin/skills/example/SKILL.md",
        ):
            content = archive.read(name)
            digest = base64.urlsafe_b64encode(hashlib.sha256(content).digest()).rstrip(b"=")
            assert records[name] == (f"sha256={digest.decode()}", str(len(content)))


def test_augment_wheel_preserves_source_when_backend_fails(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    project = _project(tmp_path)
    wheel = tmp_path / "example-1.0.0-py3-none-any.whl"
    _wheel(wheel)
    original = wheel.read_bytes()

    class FailingBackend:
        @staticmethod
        def build_wheel(wheel_directory: str) -> str:
            Path(wheel_directory, "staged").write_text("created")
            raise agent_plugins.AgentPluginError("augmentation failed")

    monkeypatch.setattr(agent_plugin_artifact, "_BACKEND", FailingBackend())

    with pytest.raises(agent_plugins.AgentPluginError, match="augmentation failed"):
        augment_wheel(wheel, project=project)

    assert wheel.read_bytes() == original


def test_augment_wheel_rejects_non_wheel(tmp_path: Path) -> None:
    source = tmp_path / "archive.zip"
    source.write_bytes(b"content")

    with pytest.raises(ValueError, match="Expected a wheel path"):
        augment_wheel(source, project=tmp_path)


def test_command_reports_unmatched_wheel(
    tmp_path: Path,
    capsys: pytest.CaptureFixture[str],
) -> None:
    missing = tmp_path / "missing.whl"

    with pytest.raises(SystemExit, match="2"):
        main([str(missing)])

    assert capsys.readouterr().err.endswith(f"no wheel files matched: {missing}\n")
