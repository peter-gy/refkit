"""Execute the authored first-success examples against installed packages."""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
from pathlib import Path

EXAMPLES = (
    ("README.md", True),
    ("packages/refkit/README.md", True),
    ("packages/polars-refkit/README.md", True),
    ("docs/get-started.md", True),
    ("docs/guides/render-output.md", True),
    ("docs/guides/edit-bibtex.md", False),
    ("docs/guides/format-bibtex.md", False),
    ("docs/guides/polars.md", False),
)
SOURCE = "@article{doe2024, author={Doe, Jane}, title={Fast Citations}, year={2024}}\n"


def check_examples(root: Path) -> None:
    for relative, compare_output in EXAMPLES:
        markdown = (root / relative).read_text(encoding="utf-8")
        example = re.search(
            r"^```python(?: \[Python\])?\n(.*?)^```", markdown, re.MULTILINE | re.DOTALL
        )
        if example is None:
            raise SystemExit(f"{relative}: missing Python example")
        with tempfile.TemporaryDirectory(prefix="refkit-doc-example-") as directory:
            Path(directory, "references.bib").write_text(SOURCE, encoding="utf-8")
            result = subprocess.run(
                [sys.executable, "-B", "-c", example.group(1)],
                cwd=directory,
                text=True,
                capture_output=True,
                check=False,
            )
        if result.returncode:
            raise SystemExit(f"{relative}: example failed\n{result.stderr}")
        if compare_output:
            expected = re.search(
                r"^```text\n(.*?)^```", markdown[example.end() :], re.MULTILINE | re.DOTALL
            )
            if expected is None or result.stdout.strip() != expected.group(1).strip():
                raise SystemExit(f"{relative}: example output differs from the documented result")
        sys.stdout.write(f"Verified {relative}\n")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Verify authored examples against installed adapters."
    )
    parser.add_argument(
        "--root", type=Path, required=True, help="Repository containing the Markdown sources"
    )
    args = parser.parse_args()
    check_examples(args.root.resolve())


if __name__ == "__main__":
    main()
