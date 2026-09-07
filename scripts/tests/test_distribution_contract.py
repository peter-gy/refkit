from __future__ import annotations

import io
import json
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path

import pytest

from scripts.distribution_contract import (
    ROOT,
    agent_plugin_violations,
    content_violations,
    distribution_paths,
    generated_members,
    internal_document_members,
)

_DIST_INFO = "refkit-1.0.0.dist-info"
_PLUGIN_ROOT = "refkit-1.0.0.agent-plugin"
_AGENT_PLUGIN_FILES = (
    "plugin.json",
    "skills/refkit/SKILL.md",
    "skills/refkit/agents/openai.yaml",
    "skills/refkit/references/contracts.md",
    "skills/refkit/references/inspect.md",
    "skills/refkit/references/render.md",
    "skills/refkit/references/edit.md",
    "skills/refkit/references/tidy.md",
)


def _wheel(path: Path, members: list[str]) -> None:
    with zipfile.ZipFile(path, "w") as archive:
        for member in members:
            archive.writestr(member, b"content")


def _sdist(path: Path, members: list[str]) -> None:
    with tarfile.open(path, "w:gz") as archive:
        for member in members:
            relative = member.partition("/.agent-plugin/")[2]
            source_root = (
                ROOT / "packages/polars-refkit/agent-plugin"
                if member.startswith("polars_refkit-")
                else ROOT
            )
            content = (
                (source_root / relative).read_bytes()
                if relative and (source_root / relative).is_file()
                else b"content"
            )
            info = tarfile.TarInfo(member)
            info.size = len(content)
            archive.addfile(info, io.BytesIO(content))


def _refkit_wheel(path: Path, plugin_files: tuple[str, ...] = _AGENT_PLUGIN_FILES) -> None:
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr(f"{_DIST_INFO}/WHEEL", "Wheel-Version: 1.0\n")
        archive.writestr(f"{_DIST_INFO}/METADATA", "Requires-Dist: agent-plugins==0.2.0\n")
        archive.writestr(
            f"{_DIST_INFO}/entry_points.txt",
            "[marimo.agent.capability]\nrefkit = refkit.agent\n",
        )
        archive.writestr(
            f"{_DIST_INFO}/agent_plugins.json",
            json.dumps({"root": _PLUGIN_ROOT, "files": list(_AGENT_PLUGIN_FILES)}),
        )
        archive.writestr("refkit/agent.py", "")
        for relative in plugin_files:
            archive.writestr(f"{_PLUGIN_ROOT}/{relative}", (ROOT / relative).read_bytes())


def _rewrite_wheel(
    path: Path,
    *,
    remove: tuple[str, ...] = (),
    add: dict[str, bytes | str] | None = None,
) -> None:
    with zipfile.ZipFile(path) as archive:
        members: dict[str, bytes | str] = {
            name: archive.read(name) for name in archive.namelist() if name not in remove
        }
    if add is not None:
        members.update(add)
    with zipfile.ZipFile(path, "w") as archive:
        for name, content in members.items():
            archive.writestr(name, content)


def _refkit_sdist_members() -> list[str]:
    root = "refkit-1.0.0"
    return [
        f"{root}/build_backend.py",
        f"{root}/src/refkit/agent.py",
        *(f"{root}/.agent-plugin/{relative}" for relative in _AGENT_PLUGIN_FILES),
    ]


def test_distribution_contract_accepts_python_sources(tmp_path: Path) -> None:
    wheel = tmp_path / "package.whl"
    _wheel(wheel, ["refkit/__init__.py", "refkit/py.typed"])

    assert generated_members(wheel) == []


def test_distribution_contract_rejects_generated_bytecode(tmp_path: Path) -> None:
    wheel = tmp_path / "package.whl"
    _wheel(
        wheel,
        [
            "refkit/__pycache__/__init__.cpython-314.pyc",
            "refkit/runtime.pyo",
        ],
    )

    assert generated_members(wheel) == [
        "refkit/__pycache__/__init__.cpython-314.pyc",
        "refkit/runtime.pyo",
    ]


def test_distribution_contract_rejects_internal_developer_documentation(
    tmp_path: Path,
) -> None:
    sdist = tmp_path / "package.tar.gz"
    _sdist(
        sdist,
        [
            "refkit/__init__.py",
            "refkit-1.0.0/development_docs/architecture.md",
        ],
    )

    assert internal_document_members(sdist) == ["refkit-1.0.0/development_docs/architecture.md"]


def test_distribution_contract_command_rejects_internal_developer_documentation(
    tmp_path: Path,
) -> None:
    sdist = tmp_path / "package.tar.gz"
    _sdist(sdist, ["refkit-1.0.0/development_docs/architecture.md"])

    result = subprocess.run(
        [sys.executable, str(ROOT / "scripts/distribution_contract.py"), str(sdist)],
        check=False,
        capture_output=True,
        text=True,
    )

    assert result.returncode == 1
    assert result.stdout == ""
    assert result.stderr == (
        "Distribution contract failed:\n"
        f"{sdist}: internal documentation member "
        "refkit-1.0.0/development_docs/architecture.md\n"
    )


def test_distribution_contract_expands_a_literal_glob(tmp_path: Path) -> None:
    wheel = tmp_path / "package.whl"
    _wheel(wheel, ["refkit/__init__.py"])

    paths, unmatched = distribution_paths([str(tmp_path / "*")])

    assert paths == [wheel]
    assert unmatched == []


def test_distribution_contract_reports_an_unmatched_glob(tmp_path: Path) -> None:
    pattern = str(tmp_path / "*.whl")

    paths, unmatched = distribution_paths([pattern])

    assert paths == []
    assert unmatched == [pattern]


def test_distribution_contract_rejects_builder_paths(tmp_path: Path) -> None:
    wheel = tmp_path / "package.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        archive.writestr("refkit/_native.so", f"source={Path.home()}/build/src/lib.rs")

    assert content_violations(wheel) == ["refkit/_native.so: embeds a builder path"]


def test_distribution_contract_rejects_local_sbom_references(tmp_path: Path) -> None:
    wheel = tmp_path / "package.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        archive.writestr(
            "package-1.0.dist-info/sboms/package.cyclonedx.json",
            b'{"bom-ref":"path+file:///build/package#package@1.0"}',
        )

    assert content_violations(wheel) == [
        "package-1.0.dist-info/sboms/package.cyclonedx.json: contains a local SBOM reference"
    ]


def test_refkit_wheel_contains_exact_agent_plugin_contract(tmp_path: Path) -> None:
    wheel = tmp_path / "refkit-1.0.0-py3-none-any.whl"
    _refkit_wheel(wheel)

    assert agent_plugin_violations(wheel) == []


def test_refkit_wheel_rejects_incomplete_agent_plugin_payload(tmp_path: Path) -> None:
    wheel = tmp_path / "refkit-1.0.0-py3-none-any.whl"
    _refkit_wheel(wheel, _AGENT_PLUGIN_FILES[:-1])

    assert agent_plugin_violations(wheel) == [
        "wheel Agent Plugin payload mismatch: "
        "missing=['refkit-1.0.0.agent-plugin/skills/refkit/references/tidy.md'], "
        "extra=[]"
    ]


@pytest.mark.parametrize(
    ("remove", "add"),
    [
        ((f"{_DIST_INFO}/WHEEL",), None),
        ((), {"other-1.0.0.dist-info/WHEEL": "Wheel-Version: 1.0\n"}),
    ],
)
def test_refkit_wheel_requires_one_dist_info(
    tmp_path: Path,
    remove: tuple[str, ...],
    add: dict[str, bytes | str] | None,
) -> None:
    wheel = tmp_path / "refkit-1.0.0-py3-none-any.whl"
    _refkit_wheel(wheel)
    _rewrite_wheel(wheel, remove=remove, add=add)

    assert agent_plugin_violations(wheel) == [
        "wheel must contain exactly one .dist-info/WHEEL file"
    ]


@pytest.mark.parametrize(
    ("marker", "expected"),
    [
        (None, f"wheel is missing {_DIST_INFO}/agent_plugins.json"),
        (b"{", f"wheel contains invalid {_DIST_INFO}/agent_plugins.json"),
        (
            b'{"root":"wrong","files":[]}',
            f"wheel contains unexpected {_DIST_INFO}/agent_plugins.json",
        ),
    ],
)
def test_refkit_wheel_validates_agent_plugin_marker(
    tmp_path: Path,
    marker: bytes | None,
    expected: str,
) -> None:
    wheel = tmp_path / "refkit-1.0.0-py3-none-any.whl"
    _refkit_wheel(wheel)
    marker_path = f"{_DIST_INFO}/agent_plugins.json"
    _rewrite_wheel(
        wheel,
        remove=(marker_path,),
        add=None if marker is None else {marker_path: marker},
    )

    assert agent_plugin_violations(wheel) == [expected]


def test_refkit_wheel_requires_agent_module(tmp_path: Path) -> None:
    wheel = tmp_path / "refkit-1.0.0-py3-none-any.whl"
    _refkit_wheel(wheel)
    _rewrite_wheel(wheel, remove=("refkit/agent.py",))

    assert agent_plugin_violations(wheel) == ["wheel is missing refkit/agent.py"]


@pytest.mark.parametrize(
    ("entry_points", "expected"),
    [
        (None, f"wheel is missing {_DIST_INFO}/entry_points.txt"),
        ("[", "wheel must register refkit = refkit.agent"),
        (
            "[marimo.agent.capability]\nrefkit = another.agent\n",
            "wheel must register refkit = refkit.agent",
        ),
    ],
)
def test_refkit_wheel_validates_capability_entry_point(
    tmp_path: Path,
    entry_points: str | None,
    expected: str,
) -> None:
    wheel = tmp_path / "refkit-1.0.0-py3-none-any.whl"
    _refkit_wheel(wheel)
    entry_points_path = f"{_DIST_INFO}/entry_points.txt"
    _rewrite_wheel(
        wheel,
        remove=(entry_points_path,),
        add=None if entry_points is None else {entry_points_path: entry_points},
    )

    assert agent_plugin_violations(wheel) == [expected]


def test_refkit_wheel_requires_agent_plugins_dependency(tmp_path: Path) -> None:
    wheel = tmp_path / "refkit-1.0.0-py3-none-any.whl"
    _refkit_wheel(wheel)
    _rewrite_wheel(
        wheel,
        add={f"{_DIST_INFO}/METADATA": "Requires-Dist: another-package\n"},
    )

    assert agent_plugin_violations(wheel) == ["wheel must require agent-plugins==0.2.0"]


def test_refkit_sdist_contains_agent_plugin_and_backend(tmp_path: Path) -> None:
    sdist = tmp_path / "refkit-1.0.0.tar.gz"
    _sdist(sdist, _refkit_sdist_members())

    assert agent_plugin_violations(sdist) == []


@pytest.mark.parametrize(
    ("missing", "expected"),
    [
        (
            "refkit-1.0.0/build_backend.py",
            "sdist is missing refkit-1.0.0/build_backend.py",
        ),
        (
            "refkit-1.0.0/src/refkit/agent.py",
            "sdist is missing refkit-1.0.0/src/refkit/agent.py",
        ),
    ],
)
def test_refkit_sdist_requires_owned_modules(
    tmp_path: Path,
    missing: str,
    expected: str,
) -> None:
    sdist = tmp_path / "refkit-1.0.0.tar.gz"
    members = [member for member in _refkit_sdist_members() if member != missing]
    _sdist(sdist, members)

    assert agent_plugin_violations(sdist) == [expected]


def test_refkit_sdist_requires_one_archive_root(tmp_path: Path) -> None:
    sdist = tmp_path / "refkit-1.0.0.tar.gz"
    _sdist(sdist, [*_refkit_sdist_members(), "another-root/file.txt"])

    assert agent_plugin_violations(sdist) == ["sdist must contain exactly one archive root"]


def test_refkit_sdist_validates_exact_agent_plugin_payload(tmp_path: Path) -> None:
    sdist = tmp_path / "refkit-1.0.0.tar.gz"
    missing = "refkit-1.0.0/.agent-plugin/skills/refkit/references/tidy.md"
    extra = "refkit-1.0.0/.agent-plugin/skills/refkit/unexpected.txt"
    members = [member for member in _refkit_sdist_members() if member != missing]
    _sdist(sdist, [*members, extra])

    assert agent_plugin_violations(sdist) == [
        f"sdist Agent Plugin payload mismatch: missing=['{missing}'], extra=['{extra}']"
    ]


@pytest.mark.parametrize("archive_format", ["wheel", "sdist"])
def test_polars_distribution_carries_its_agent_skill(tmp_path: Path, archive_format: str) -> None:
    files = [
        "plugin.json",
        "skills/polars-refkit/SKILL.md",
        "skills/polars-refkit/agents/openai.yaml",
        "skills/polars-refkit/references/inspect.md",
        "skills/polars-refkit/references/render.md",
        "skills/polars-refkit/references/tidy.md",
    ]
    if archive_format == "wheel":
        path = tmp_path / "polars_refkit-1.0.0-py3-none-any.whl"
        dist_info = "polars_refkit-1.0.0.dist-info"
        plugin_root = "polars_refkit-1.0.0.agent-plugin"
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr(f"{dist_info}/WHEEL", "Wheel-Version: 1.0\n")
            archive.writestr(f"{dist_info}/METADATA", "Requires-Dist: agent-plugins==0.2.0\n")
            archive.writestr(
                f"{dist_info}/entry_points.txt",
                "[marimo.agent.capability]\npolars_refkit = polars_refkit.agent\n",
            )
            archive.writestr(
                f"{dist_info}/agent_plugins.json", json.dumps({"root": plugin_root, "files": files})
            )
            archive.writestr("polars_refkit/agent.py", "")
            for name in files:
                archive.writestr(
                    f"{plugin_root}/{name}",
                    (ROOT / "packages/polars-refkit/agent-plugin" / name).read_bytes(),
                )
    else:
        path = tmp_path / "polars_refkit-1.0.0.tar.gz"
        _sdist(
            path,
            [
                "polars_refkit-1.0.0/build_backend.py",
                "polars_refkit-1.0.0/polars_refkit/agent.py",
                *(f"polars_refkit-1.0.0/.agent-plugin/{name}" for name in files),
            ],
        )

    assert agent_plugin_violations(path) == []


def test_agent_payload_must_match_release_sources(tmp_path: Path) -> None:
    wheel = tmp_path / "refkit-1.0.0-py3-none-any.whl"
    _refkit_wheel(wheel)
    member = f"{_PLUGIN_ROOT}/skills/refkit/references/render.md"
    _rewrite_wheel(wheel, add={member: "outdated resource"})

    assert agent_plugin_violations(wheel) == [
        f"{member}: Agent Plugin resource differs from release source"
    ]


def test_agent_marker_accepts_resource_order_from_the_builder(tmp_path: Path) -> None:
    wheel = tmp_path / "refkit-1.0.0-py3-none-any.whl"
    _refkit_wheel(wheel)
    files = [
        "skills/refkit/references/tidy.md",
        "skills/refkit/references/render.md",
        "skills/refkit/references/inspect.md",
        "skills/refkit/references/edit.md",
        "skills/refkit/references/contracts.md",
        "skills/refkit/agents/openai.yaml",
        "skills/refkit/SKILL.md",
        "plugin.json",
    ]
    _rewrite_wheel(
        wheel,
        add={
            f"{_DIST_INFO}/agent_plugins.json": json.dumps({"root": _PLUGIN_ROOT, "files": files})
        },
    )

    assert agent_plugin_violations(wheel) == []


def test_agent_marker_rejects_duplicate_resources(tmp_path: Path) -> None:
    wheel = tmp_path / "refkit-1.0.0-py3-none-any.whl"
    _refkit_wheel(wheel)
    _rewrite_wheel(
        wheel,
        add={
            f"{_DIST_INFO}/agent_plugins.json": json.dumps(
                {"root": _PLUGIN_ROOT, "files": [*_AGENT_PLUGIN_FILES, "plugin.json"]}
            )
        },
    )

    assert agent_plugin_violations(wheel) == [
        f"wheel contains unexpected {_DIST_INFO}/agent_plugins.json"
    ]
