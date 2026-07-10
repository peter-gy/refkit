from __future__ import annotations

import base64
import csv
import hashlib
import io
import json
import zipfile
from pathlib import Path

from scripts.distribution_contract import content_violations
from scripts.normalize_wheel import normalize_wheel


def _wheel(path: Path, sbom: dict[str, object]) -> tuple[str, str]:
    sbom_name = "package-1.0.dist-info/sboms/package.cyclonedx.json"
    record_name = "package-1.0.dist-info/RECORD"
    sbom_content = (json.dumps(sbom) + "\n").encode()
    rows = [
        [sbom_name, "sha256=original", str(len(sbom_content))],
        [record_name, "", ""],
    ]
    record = io.StringIO(newline="")
    csv.writer(record, lineterminator="\n").writerows(rows)
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr(sbom_name, sbom_content)
        archive.writestr(record_name, record.getvalue())
    return sbom_name, record_name


def test_normalize_wheel_replaces_local_sbom_references(tmp_path: Path) -> None:
    wheel = tmp_path / "package.whl"
    root_ref = "path+file:///build/package#package@1.0"
    dependency_ref = "path+file:///build/core#1.0"
    sbom_name, record_name = _wheel(
        wheel,
        {
            "bomFormat": "CycloneDX",
            "specVersion": "1.5",
            "serialNumber": "urn:uuid:builder-specific",
            "metadata": {
                "timestamp": "2026-07-10T00:00:00Z",
                "component": {
                    "bom-ref": root_ref,
                    "name": "package",
                    "version": "1.0",
                    "purl": "pkg:cargo/package@1.0?download_url=file://.",
                },
            },
            "components": [
                {
                    "bom-ref": dependency_ref,
                    "name": "core",
                    "version": "1.0",
                    "purl": "pkg:cargo/core@1.0?download_url=file://../core",
                }
            ],
            "dependencies": [{"ref": root_ref, "dependsOn": [dependency_ref]}],
        },
    )

    assert normalize_wheel(wheel) == 1

    with zipfile.ZipFile(wheel) as archive:
        normalized_content = archive.read(sbom_name)
        normalized = json.loads(normalized_content)
        record_content = io.StringIO(archive.read(record_name).decode())
        records = {row[0]: row for row in csv.reader(record_content)}

    assert "serialNumber" not in normalized
    assert "timestamp" not in normalized["metadata"]
    assert normalized["metadata"]["component"]["bom-ref"] == "pkg:cargo/package@1.0"
    assert normalized["components"][0]["bom-ref"] == "pkg:cargo/core@1.0"
    assert normalized["dependencies"] == [
        {"ref": "pkg:cargo/package@1.0", "dependsOn": ["pkg:cargo/core@1.0"]}
    ]
    expected_digest = base64.urlsafe_b64encode(hashlib.sha256(normalized_content).digest()).rstrip(
        b"="
    )
    assert records[sbom_name] == [
        sbom_name,
        f"sha256={expected_digest.decode()}",
        str(len(normalized_content)),
    ]
    assert content_violations(wheel) == []


def test_normalize_wheel_leaves_a_wheel_without_sboms_unchanged(tmp_path: Path) -> None:
    wheel = tmp_path / "package.whl"
    with zipfile.ZipFile(wheel, "w") as archive:
        archive.writestr("package/__init__.py", "")

    before = wheel.read_bytes()

    assert normalize_wheel(wheel) == 0
    assert wheel.read_bytes() == before
