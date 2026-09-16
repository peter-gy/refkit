from __future__ import annotations

import subprocess
from pathlib import Path

import pytest

from scripts.docs_contract import check_contract


def _write(path: Path, source: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(source, encoding="utf-8")


def _documentation_tree(root: Path) -> None:
    _write(root / "README.md", "[Contributing](development_docs/README.md)\n")
    _write(root / "AGENTS.md", "[Developer docs](development_docs/README.md)\n")
    _write(root / "docs/guide.md", "[API](../packages/refkit/README.md)\n")
    _write(root / "development_docs/README.md", "[Architecture](architecture.md)\n")
    _write(root / "development_docs/architecture.md", "# Architecture\n")
    _write(root / "packages/refkit/README.md", "# refkit\n")


def test_documentation_contract_reports_missing_and_escaping_targets(tmp_path: Path) -> None:
    _documentation_tree(tmp_path)
    _write(
        tmp_path / "docs/guide.md",
        "[Missing](missing.md)\n[Outside](../../outside.md)\n",
    )

    assert check_contract(tmp_path) == [
        "docs/guide.md:1: local link target does not exist: missing.md",
        "docs/guide.md:2: local link escapes the repository: ../../outside.md",
    ]


@pytest.mark.parametrize(
    "source",
    [
        "[Page][reference]\n\n[reference]: missing.md\n",
        "- Resources\n    - [Page](missing.md)\n",
        "- Resources\n\t- [Page](missing.md)\n",
    ],
    ids=["reference", "nested-list", "tab-indented-list"],
)
def test_documentation_links_resolve_inside_markdown_structures(
    tmp_path: Path, source: str
) -> None:
    _documentation_tree(tmp_path)
    _write(tmp_path / "docs/guide.md", source)

    errors = check_contract(tmp_path)
    assert len(errors) == 1
    assert "local link target does not exist: missing.md" in errors[0]


def test_documentation_contract_resolves_vitepress_routes_and_public_assets(
    tmp_path: Path,
) -> None:
    _documentation_tree(tmp_path)
    _write(tmp_path / "docs/get-started.md", "# Get started\n")
    _write(tmp_path / "docs/public/brand.svg", "<svg/>\n")
    _write(
        tmp_path / "docs/guide.md",
        "[Start](/get-started)\n[Brand](/brand.svg)\n",
    )

    assert check_contract(tmp_path) == []


def test_root_readme_can_link_to_index_and_fenced_links_are_ignored(tmp_path: Path) -> None:
    _documentation_tree(tmp_path)
    _write(
        tmp_path / "README.md",
        "[Contributing](development_docs/README.md)\n"
        "[Encoded](development_docs%2FREADME.md?view=1#top)\n"
        "[Section](#install)\n"
        "[Project](https://example.com/refkit)\n"
        "[^note]: A prose footnote.\n"
        "Use `[Inline](missing-inline.md)` as syntax.\n"
        "    [Indented](missing-indented.md)\n"
        "\t[Tabbed](missing-tabbed.md)\n"
        "```md\n[Example](missing.md)\n```\n",
    )

    assert check_contract(tmp_path) == []


def test_documentation_contract_ignores_private_git_ignored_markdown(tmp_path: Path) -> None:
    _documentation_tree(tmp_path)
    subprocess.run(["git", "init", "-q"], cwd=tmp_path, check=True)
    _write(tmp_path / ".gitignore", "LOCAL_NOTES.md\n")
    _write(tmp_path / "LOCAL_NOTES.md", "[Local](missing.md)\n")

    assert check_contract(tmp_path) == []
