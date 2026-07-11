from __future__ import annotations

import io
import subprocess
import sys
import tarfile
import zipfile
from pathlib import Path

from scripts.distribution_contract import (
    ROOT,
    content_violations,
    distribution_paths,
    generated_members,
    internal_document_members,
)


def _wheel(path: Path, members: list[str]) -> None:
    with zipfile.ZipFile(path, "w") as archive:
        for member in members:
            archive.writestr(member, b"content")


def _sdist(path: Path, members: list[str]) -> None:
    with tarfile.open(path, "w:gz") as archive:
        for member in members:
            content = b"content"
            info = tarfile.TarInfo(member)
            info.size = len(content)
            archive.addfile(info, io.BytesIO(content))


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
