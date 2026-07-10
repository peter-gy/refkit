from __future__ import annotations

import argparse
import base64
import csv
import hashlib
import io
import json
import os
import sys
import zipfile
from pathlib import Path
from typing import Any

from scripts.distribution_contract import distribution_paths

LOCAL_REFERENCE_PREFIX = "path+file://"
LOCAL_DOWNLOAD_QUALIFIER = "?download_url=file://"


def _normalize_purl(value: str) -> str:
    prefix, separator, local_reference = value.partition(LOCAL_DOWNLOAD_QUALIFIER)
    if not separator:
        return value
    _, fragment_separator, fragment = local_reference.partition("#")
    return prefix + (f"#{fragment}" if fragment_separator else "")


def _collect_reference_map(value: Any, references: dict[str, str]) -> None:
    if isinstance(value, dict):
        bom_ref = value.get("bom-ref")
        if isinstance(bom_ref, str) and bom_ref.startswith(LOCAL_REFERENCE_PREFIX):
            purl = value.get("purl")
            if isinstance(purl, str):
                references[bom_ref] = _normalize_purl(purl)
            else:
                name = value.get("name")
                version = value.get("version")
                if not isinstance(name, str) or not isinstance(version, str):
                    raise ValueError(f"cannot normalize SBOM reference {bom_ref}")
                references[bom_ref] = f"pkg:cargo/{name}@{version}"
        for child in value.values():
            _collect_reference_map(child, references)
    elif isinstance(value, list):
        for child in value:
            _collect_reference_map(child, references)


def _replace_local_references(value: Any, references: dict[str, str]) -> Any:
    if isinstance(value, dict):
        return {key: _replace_local_references(child, references) for key, child in value.items()}
    if isinstance(value, list):
        return [_replace_local_references(child, references) for child in value]
    if isinstance(value, str):
        if value in references:
            return references[value]
        if value.startswith(LOCAL_REFERENCE_PREFIX):
            raise ValueError(f"unmapped local SBOM reference {value}")
        return _normalize_purl(value)
    return value


def normalize_sbom(content: bytes) -> bytes:
    document = json.loads(content)
    references: dict[str, str] = {}
    _collect_reference_map(document, references)
    normalized = _replace_local_references(document, references)
    normalized.pop("serialNumber", None)
    metadata = normalized.get("metadata")
    if isinstance(metadata, dict):
        metadata.pop("timestamp", None)
    return (json.dumps(normalized, indent=2, ensure_ascii=False) + "\n").encode()


def _record_content(original: bytes, replacements: dict[str, bytes]) -> bytes:
    rows = list(csv.reader(io.StringIO(original.decode())))
    seen = set()
    for row in rows:
        if row[0] not in replacements:
            continue
        content = replacements[row[0]]
        digest = base64.urlsafe_b64encode(hashlib.sha256(content).digest()).rstrip(b"=")
        row[1] = f"sha256={digest.decode()}"
        row[2] = str(len(content))
        seen.add(row[0])
    missing = replacements.keys() - seen
    if missing:
        raise ValueError(f"wheel RECORD is missing {', '.join(sorted(missing))}")
    output = io.StringIO(newline="")
    csv.writer(output, lineterminator="\n").writerows(rows)
    return output.getvalue().encode()


def normalize_wheel(path: Path) -> int:
    if path.suffix != ".whl":
        raise ValueError(f"expected a wheel: {path}")
    with zipfile.ZipFile(path) as archive:
        infos = archive.infolist()
        contents = {info.filename: archive.read(info.filename) for info in infos}

    replacements = {
        name: normalize_sbom(content)
        for name, content in contents.items()
        if ".dist-info/sboms/" in name and name.endswith(".json")
    }
    if not replacements:
        return 0

    record_names = [name for name in contents if name.endswith(".dist-info/RECORD")]
    if len(record_names) != 1:
        raise ValueError(f"expected one wheel RECORD in {path}")
    record_name = record_names[0]
    replacements[record_name] = _record_content(contents[record_name], replacements)

    temporary = path.with_name(f".{path.name}.tmp")
    try:
        with zipfile.ZipFile(temporary, "w") as destination:
            for info in infos:
                destination.writestr(info, replacements.get(info.filename, contents[info.filename]))
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)
    return len(replacements) - 1


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Normalize generated SBOM references in wheel metadata."
    )
    parser.add_argument("wheels", nargs="+")
    return parser.parse_args()


def main() -> int:
    wheels, unmatched = distribution_paths(_parse_args().wheels)
    errors = [f"no wheel files matched {argument}" for argument in unmatched]
    normalized = 0
    for wheel in wheels:
        try:
            normalized += normalize_wheel(wheel)
        except (OSError, ValueError, zipfile.BadZipFile) as error:
            errors.append(str(error))
    if errors:
        sys.stderr.write("Wheel normalization failed:\n" + "\n".join(errors) + "\n")
        return 1
    sys.stdout.write(f"Normalized {normalized} wheel SBOMs\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
