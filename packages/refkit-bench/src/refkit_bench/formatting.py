from __future__ import annotations

import json
import re
import subprocess
from collections import Counter
from dataclasses import dataclass, field, replace
from hashlib import sha256
from itertools import zip_longest
from pathlib import Path
from queue import Empty, Queue
from threading import Thread
from typing import Any, cast

from refkit_bench.fixtures import Workload
from refkit_bench.model import Prepared

_DATA = Path(__file__).with_name("data") / "bibtex-tidy"


@dataclass(frozen=True)
class FormatCase:
    name: str
    input: str
    options: dict[str, Any]
    expected: str
    dimensions: tuple[str, ...] = ()
    metadata: dict[str, str | int | float] = field(default_factory=dict)


def formatting_cases() -> tuple[FormatCase, ...]:
    document = json.loads((_DATA / "cases.json").read_text(encoding="utf-8"))
    result = []
    for row in document["cases"]:
        options = json.dumps(row["options"], sort_keys=True, separators=(",", ":"))
        if sha256(options.encode()).hexdigest() != row["options_sha256"]:
            raise ValueError(f"Formatter fixture {row['name']} has an invalid options hash")
        for name in ("input", "expected"):
            if sha256(row[name].encode()).hexdigest() != row[f"{name}_sha256"]:
                raise ValueError(f"Formatter fixture {row['name']} has an invalid {name} hash")
        result.append(
            FormatCase(
                name=row["name"],
                input=row["input"],
                options=row["options"],
                expected=row["expected"],
                dimensions=tuple(row["dimensions"]),
                metadata={
                    "source_repository": document["repository"],
                    "source_revision": document["revision"],
                    "source_license": document["license"],
                    "source_path": row["source_path"],
                    "source_document": row["source_document"],
                    "source_sha256": row["source_sha256"],
                },
            )
        )
    return tuple(result)


def layout_case(workload: Workload) -> FormatCase:
    options = {
        "align": 1,
        "space": 2,
        "escape": False,
        "removeDuplicateFields": False,
        "lowercase": True,
        "trailingCommas": True,
        "blankLines": True,
    }
    entries = []
    for record in workload.records:
        conference = record.item_type == "paper-conference"
        kind = "inproceedings" if conference else "article"
        authors = " and ".join(
            f"{family}, {given}" if given else family
            for family, given in (record.authors or ((record.family, record.given),))
        )
        fields = [
            ("author", authors),
            ("title", record.raw_title or record.title),
            ("booktitle" if conference else "journal", record.container),
            ("year", str(record.year)),
        ]
        if record.volume is not None:
            fields.append(("volume", str(record.volume)))
        if record.page_range:
            fields.append(("pages", re.sub(r"(?<=\d)-+(?=\d)", "--", record.page_range)))
        if record.doi:
            fields.append(("doi", record.doi))
        body = "\n".join(f"  {name} = {{{value}}}," for name, value in fields)
        entries.append(f"@{kind}{{{record.key},\n{body}\n}}")
    return FormatCase(
        name=f"layout-{workload.size}",
        input=workload.bibtex,
        options=options,
        expected="\n\n".join(entries) + "\n",
        dimensions=("layout", "scaling"),
        metadata={"source_license": workload.source_license_name},
    )


def key_case(workload: Workload) -> FormatCase:
    totals = Counter(record.year for record in workload.records)
    seen: Counter[int] = Counter()
    records = []
    for record in workload.records:
        seen[record.year] += 1
        number = seen[record.year]
        suffix = ""
        if totals[record.year] > 1:
            while number:
                number, digit = divmod(number - 1, 26)
                suffix = chr(ord("a") + digit) + suffix
        records.append(replace(record, key=f"{record.year}{suffix}"))
    expected = layout_case(replace(workload, records=tuple(records))).expected
    case = layout_case(workload)
    return replace(
        case,
        name=f"keys-{workload.size}",
        options={**case.options, "generateKeys": "[year]"},
        expected=expected,
        dimensions=("key-generation", "collisions", "scaling"),
    )


def prepare_format(case: FormatCase, package: str) -> Prepared:
    metadata: dict[str, str | int | float] = {
        **case.metadata,
        "input_sha256": sha256(case.input.encode()).hexdigest(),
        "input_bytes": len(case.input.encode()),
        "expected_sha256": sha256(case.expected.encode()).hexdigest(),
        "options_json": json.dumps(case.options, sort_keys=True, separators=(",", ":")),
        "dimensions": ",".join(case.dimensions),
        "measured_phase": "parse-transform-format",
    }

    def check(value: object) -> None:
        actual = (
            cast(dict[str, Any], value)["bibtex"]
            if isinstance(value, dict)
            else getattr(value, "bibtex", None)
        )
        if actual != case.expected:
            difference = next(
                (
                    f"line {index}: expected {expected!r}, got {observed!r}"
                    for index, (expected, observed) in enumerate(
                        zip_longest(
                            case.expected.splitlines(keepends=True),
                            str(actual).splitlines(keepends=True),
                        ),
                        start=1,
                    )
                    if expected != observed
                ),
                "output type differs",
            )
            raise AssertionError(
                f"{package} failed exact formatter output for {case.name}: {difference}"
            )

    if package == "refkit":
        import refkit

        options = refkit.TidyOptions(
            **{
                re.sub(r"(?<!^)(?=[A-Z])", "_", name).lower(): value
                for name, value in case.options.items()
            }
        )
        return Prepared(
            operation=lambda: refkit.tidy_bibtex(case.input, options=options),
            check=check,
            metadata=metadata,
        )
    if package != "bibtex-tidy":
        raise ValueError(f"Unknown formatter participant: {package}")
    worker = _NodeWorker()
    try:
        worker.request({"input": case.input, "options": case.options})
    except BaseException:
        worker.close()
        raise

    def measure(loops: int) -> tuple[float, object]:
        response = worker.request({"loops": loops})
        return float(response["elapsed"]), response["result"]

    return Prepared(
        operation=lambda: measure(1)[1],
        check=check,
        metadata={**metadata, **worker.metadata},
        measure=measure,
        close=worker.close,
    )


class _NodeWorker:
    def __init__(self) -> None:
        module = (
            Path(__file__).resolve().parents[2] / "node/node_modules/bibtex-tidy/bibtex-tidy.js"
        )
        if not module.is_file():
            raise RuntimeError(
                "Install the Node participant with npm ci --ignore-scripts "
                "--prefix packages/refkit-bench/node"
            )
        self.process = subprocess.Popen(
            ["node", str(Path(__file__).with_name("node_worker.mjs")), str(module)],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            encoding="utf-8",
        )
        self.responses: Queue[str] = Queue()
        self.reader = Thread(target=self._read, daemon=True)
        self.reader.start()
        try:
            self.metadata = self._response()
            if self.metadata.get("package_version") != "1.14.0":
                raise RuntimeError("The Node participant requires bibtex-tidy 1.14.0")
        except BaseException:
            self.close()
            raise

    def _read(self) -> None:
        assert self.process.stdout is not None
        for line in self.process.stdout:
            self.responses.put(line)
        self.responses.put("")

    def _response(self) -> dict[str, Any]:
        try:
            line = self.responses.get(timeout=60)
            if not line:
                raise RuntimeError("The Node formatter worker exited before responding")
            response: dict[str, Any] = json.loads(line)
            if not isinstance(response, dict):
                raise ValueError("expected a JSON object from the Node worker")
            if "error" in response:
                raise RuntimeError(f"Node formatter failed: {response['error']}")
            return response
        except (Empty, ValueError, RuntimeError) as exc:
            self.close()
            raise RuntimeError(f"Node formatter protocol failed: {exc}") from exc

    def request(self, value: dict[str, Any]) -> dict[str, Any]:
        assert self.process.stdin is not None
        try:
            self.process.stdin.write(json.dumps(value) + "\n")
            self.process.stdin.flush()
        except (BrokenPipeError, OSError) as exc:
            self.close()
            raise RuntimeError("The Node formatter worker input is closed") from exc
        return self._response()

    def close(self) -> None:
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait(timeout=2)
        self.reader.join(timeout=2)
        if self.process.stdin is not None:
            self.process.stdin.close()
        if self.process.stdout is not None:
            self.process.stdout.close()
