from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
DEVELOPMENT_DOCS = Path("development_docs")
DEVELOPMENT_INDEX = DEVELOPMENT_DOCS / "README.md"
MARKDOWN_ROOTS = (
    Path(".github"),
    Path("crates"),
    Path("docs"),
    DEVELOPMENT_DOCS,
    Path("packages"),
    Path("scripts"),
    Path("testdata"),
)
PUBLIC_DOCS = (
    Path("README.md"),
    Path("packages/refkit/README.md"),
    Path("packages/refkit-core/README.md"),
    Path("packages/polars-refkit/README.md"),
)
SKIPPED_DIRECTORIES = {
    ".git",
    ".mypy_cache",
    ".nox",
    ".pytest_cache",
    ".pyrefly",
    ".ruff_cache",
    ".tox",
    ".ty",
    ".venv",
    "__pycache__",
    "build",
    "dist",
    "node_modules",
    "target",
}
LINK = re.compile(r"!?\[[^\]]*\]\(\s*(?P<target><[^>\n]+>|[^\s)\n]+)")
LIST_ITEM = re.compile(r"(?:[-+*]|\d{1,9}[.)])\s+")
REFERENCE_LINK = re.compile(r"^\s{0,3}\[(?!\^)[^\]]+\]:\s*(?P<target><[^>\n]+>|[^\s\n]+)")


def _git_markdown_files(root: Path) -> list[Path] | None:
    try:
        result = subprocess.run(
            [
                "git",
                "-C",
                str(root),
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
                "--",
                "*.md",
            ],
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError:
        return None
    if result.returncode != 0:
        return None
    return sorted(
        path for value in result.stdout.split("\0") if value and (path := root / value).is_file()
    )


def _markdown_files(root: Path) -> list[Path]:
    if (files := _git_markdown_files(root)) is not None:
        return files
    files = list(root.glob("*.md"))
    for relative_root in MARKDOWN_ROOTS:
        start = root / relative_root
        if not start.is_dir():
            continue
        for directory, names, filenames in os.walk(start):
            names[:] = sorted(name for name in names if name not in SKIPPED_DIRECTORIES)
            files.extend(
                Path(directory) / filename
                for filename in sorted(filenames)
                if filename.endswith(".md")
            )
    return sorted(set(files))


def _without_inline_code(line: str) -> str:
    visible = []
    position = 0
    while (start := line.find("`", position)) >= 0:
        visible.append(line[position:start])
        end_of_marker = start
        while end_of_marker < len(line) and line[end_of_marker] == "`":
            end_of_marker += 1
        marker = line[start:end_of_marker]
        end = line.find(marker, end_of_marker)
        if end < 0:
            visible.append(line[start:])
            return "".join(visible)
        visible.append(" " * (end + len(marker) - start))
        position = end + len(marker)
    visible.append(line[position:])
    return "".join(visible)


def _links(path: Path) -> list[tuple[int, str]]:
    links = []
    fence: str | None = None
    list_content_indents: list[int] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        expanded = line.expandtabs(4)
        indentation = len(expanded) - len(expanded.lstrip(" "))
        stripped = expanded[indentation:]
        marker = next((value for value in ("```", "~~~") if stripped.startswith(value)), None)
        if fence is not None:
            if marker == fence:
                fence = None
            continue
        if marker is not None:
            fence = marker
            continue
        if not stripped:
            continue
        while list_content_indents and indentation < list_content_indents[-1]:
            list_content_indents.pop()
        list_item = LIST_ITEM.match(stripped)
        code_indent = list_content_indents[-1] + 4 if list_content_indents else 4
        if indentation >= code_indent and list_item is None:
            continue
        prose = _without_inline_code(expanded)
        links.extend(
            (line_number, match.group("target").strip("<>")) for match in LINK.finditer(prose)
        )
        if match := REFERENCE_LINK.match(prose):
            links.append((line_number, match.group("target").strip("<>")))
        if list_item is not None:
            content_indent = indentation + list_item.end()
            if not list_content_indents or content_indent > list_content_indents[-1]:
                list_content_indents.append(content_indent)
    return links


def _local_target(source: Path, target: str) -> Path | None:
    parsed = urlsplit(target)
    if parsed.scheme or parsed.netloc or not parsed.path:
        return None
    decoded = unquote(parsed.path)
    return (source.parent / decoded).resolve()


def _is_within(path: Path, directory: Path) -> bool:
    try:
        path.relative_to(directory)
    except ValueError:
        return False
    return True


def _public_docs(root: Path) -> set[Path]:
    files = {root / path for path in PUBLIC_DOCS if (root / path).is_file()}
    docs = root / "docs"
    if docs.is_dir():
        files.update(docs.rglob("*.md"))
    return files


def check_contract(root: Path = ROOT) -> list[str]:
    root = root.resolve()
    development_docs = root / DEVELOPMENT_DOCS
    errors = []

    if not (root / DEVELOPMENT_INDEX).is_file():
        errors.append(f"missing developer documentation index: {DEVELOPMENT_INDEX}")

    if development_docs.is_dir():
        errors.extend(
            f"developer documentation must be Markdown: {path.relative_to(root)}"
            for path in sorted(path for path in development_docs.rglob("*") if path.is_file())
            if path.suffix != ".md"
        )

    public_docs = _public_docs(root)
    for source in _markdown_files(root):
        for line_number, target in _links(source):
            resolved = _local_target(source, target)
            if resolved is None:
                continue
            location = f"{source.relative_to(root)}:{line_number}"
            if not _is_within(resolved, root):
                errors.append(f"{location}: local link escapes the repository: {target}")
                continue
            if not resolved.exists():
                errors.append(f"{location}: local link target does not exist: {target}")
                continue
            if source in public_docs and _is_within(resolved, development_docs):
                allowed = source == root / "README.md" and resolved == root / DEVELOPMENT_INDEX
                if not allowed:
                    errors.append(
                        f"{location}: end-user documentation links to developer documentation: "
                        f"{target}"
                    )

    return errors


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Validate RefKit documentation boundaries.")
    parser.add_argument(
        "--root",
        type=Path,
        default=ROOT,
        help="Repository root. Defaults to the checkout containing this script.",
    )
    return parser.parse_args()


def main() -> None:
    errors = check_contract(_parse_args().root)
    if errors:
        raise SystemExit(
            "Documentation contract failed:\n" + "\n".join(f"- {error}" for error in errors)
        )
    sys.stdout.write("Documentation contract passed\n")


if __name__ == "__main__":
    main()
