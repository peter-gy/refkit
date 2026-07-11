from __future__ import annotations

import subprocess
from pathlib import Path

from scripts.docs_contract import ROOT, check_contract


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


def test_repository_matches_the_documentation_contract() -> None:
    assert check_contract(ROOT) == []


def test_development_docs_reject_non_markdown_files(tmp_path: Path) -> None:
    _documentation_tree(tmp_path)
    _write(tmp_path / "development_docs/state.json", "{}\n")

    assert check_contract(tmp_path) == [
        "developer documentation must be Markdown: development_docs/state.json"
    ]


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


def test_documentation_contract_checks_scripts_guidance(tmp_path: Path) -> None:
    _documentation_tree(tmp_path)
    _write(tmp_path / "scripts/AGENTS.md", "[Missing](missing.md)\n")

    assert check_contract(tmp_path) == [
        "scripts/AGENTS.md:1: local link target does not exist: missing.md"
    ]


def test_public_docs_cannot_link_to_developer_documentation(tmp_path: Path) -> None:
    _documentation_tree(tmp_path)
    _write(
        tmp_path / "packages/refkit/README.md",
        "[Internals](../../development_docs/architecture.md)\n",
    )

    assert check_contract(tmp_path) == [
        "packages/refkit/README.md:1: end-user documentation links to developer documentation: "
        "../../development_docs/architecture.md"
    ]


def test_reference_links_cannot_bypass_the_public_docs_boundary(tmp_path: Path) -> None:
    _documentation_tree(tmp_path)
    _write(
        tmp_path / "packages/refkit/README.md",
        "[Internals][developer]\n\n[developer]: ../../development_docs/architecture.md\n",
    )

    assert check_contract(tmp_path) == [
        "packages/refkit/README.md:3: end-user documentation links to developer documentation: "
        "../../development_docs/architecture.md"
    ]


def test_nested_list_links_cannot_bypass_the_public_docs_boundary(tmp_path: Path) -> None:
    _documentation_tree(tmp_path)
    _write(
        tmp_path / "packages/refkit/README.md",
        "- Resources\n    - [Internals](../../development_docs/architecture.md)\n",
    )

    assert check_contract(tmp_path) == [
        "packages/refkit/README.md:2: end-user documentation links to developer documentation: "
        "../../development_docs/architecture.md"
    ]


def test_tab_nested_list_links_cannot_bypass_the_public_docs_boundary(tmp_path: Path) -> None:
    _documentation_tree(tmp_path)
    _write(
        tmp_path / "packages/refkit/README.md",
        "- Resources\n\t- [Internals](../../development_docs/architecture.md)\n",
    )

    assert check_contract(tmp_path) == [
        "packages/refkit/README.md:2: end-user documentation links to developer documentation: "
        "../../development_docs/architecture.md"
    ]


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


def test_documentation_contract_requires_the_developer_index(tmp_path: Path) -> None:
    _write(tmp_path / "README.md", "# refkit\n")

    assert check_contract(tmp_path) == [
        "missing developer documentation index: development_docs/README.md"
    ]


def test_documentation_contract_ignores_private_git_ignored_markdown(tmp_path: Path) -> None:
    _documentation_tree(tmp_path)
    subprocess.run(["git", "init", "-q"], cwd=tmp_path, check=True)
    _write(tmp_path / ".gitignore", "CONTEXT.md\n")
    _write(tmp_path / "CONTEXT.md", "[Local](missing.md)\n")

    assert check_contract(tmp_path) == []
