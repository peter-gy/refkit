from __future__ import annotations

import ast
import inspect
from pathlib import Path
from typing import Any, cast

import pytest

import refkit as rk
import refkit_core

WORKSPACE = Path(__file__).parents[2]
CORE_STUB = WORKSPACE / "refkit-core" / "src" / "refkit_core" / "_refkit_core.pyi"
EXPECTED_TIDY_OPTION_NAMES = (
    "omit",
    "curly",
    "numeric",
    "months",
    "space",
    "tab",
    "align",
    "blank_lines",
    "sort",
    "duplicates",
    "merge",
    "strip_enclosing_braces",
    "drop_all_caps",
    "escape",
    "sort_fields",
    "strip_comments",
    "trailing_commas",
    "encode_urls",
    "tidy_comments",
    "remove_empty_fields",
    "remove_duplicate_fields",
    "generate_keys",
    "max_authors",
    "lowercase",
    "enclosing_braces",
    "remove_braces",
    "wrap",
)


def _stub_tidy_option_names() -> tuple[str, ...]:
    module = ast.parse(CORE_STUB.read_text(encoding="utf-8"))
    for node in module.body:
        if isinstance(node, ast.ClassDef) and node.name == "TidyOptions":
            init = next(
                item
                for item in node.body
                if isinstance(item, ast.FunctionDef) and item.name == "__init__"
            )
            return tuple(arg.arg for arg in init.args.kwonlyargs)
    raise AssertionError("TidyOptions stub not found")


def test_tidy_options_stub_lists_public_keywords() -> None:
    assert _stub_tidy_option_names() == EXPECTED_TIDY_OPTION_NAMES


def test_tidy_options_native_allowlist_lists_public_keywords() -> None:
    assert tuple(refkit_core._tidy_option_names) == EXPECTED_TIDY_OPTION_NAMES


def test_tidy_options_runtime_signature_lists_public_keywords() -> None:
    signature = inspect.signature(rk.TidyOptions)

    assert tuple(signature.parameters) == EXPECTED_TIDY_OPTION_NAMES
    assert signature.parameters["space"].default == 2
    assert signature.parameters["escape"].default is True
    assert signature.parameters["sort_fields"].default is None


def test_tidy_options_reject_unknown_names() -> None:
    options_type = cast(Any, rk.TidyOptions)
    with pytest.raises(ValueError, match="unknown tidy option"):
        options_type(unknown=True)


@pytest.mark.parametrize(
    ("option", "value", "message"),
    [
        ("sort_fields", "title", "iterable of strings"),
        ("duplicates", ["bogus"], "unknown duplicate rule"),
        ("merge", "bogus", "unknown merge strategy"),
        ("wrap", "wide", "integer"),
        ("space", "two", "integer"),
        ("space", True, "integer"),
        ("max_authors", False, "integer"),
    ],
)
def test_tidy_options_validate_representative_values(
    option: str,
    value: object,
    message: str,
) -> None:
    options_type = cast(Any, rk.TidyOptions)
    with pytest.raises((TypeError, ValueError), match=message):
        options_type(**{option: value})


def test_tidy_options_default_toggles_forward_to_formatter() -> None:
    source = """@article{doe2024,
  year={2024},
  title={Fast Citations},
  author={Doe, Jane}
}
"""

    sorted_fields = rk.tidy_bibtex(source, options=rk.TidyOptions(sort_fields=True))
    assert sorted_fields.bibtex.index("title") < sorted_fields.bibtex.index("author")

    wrapped = rk.tidy_bibtex(
        (
            "@article{wide, title={One two three four five six seven eight nine "
            "ten eleven twelve thirteen fourteen fifteen sixteen}}\n"
        ),
        options=rk.TidyOptions(wrap=True),
    )
    assert "\n    One two" in wrapped.bibtex

    generated = rk.tidy_bibtex(
        "@article{old, author={Doe, Jane}, title={Fast Citations}, year={2024}}\n",
        options=rk.TidyOptions(generate_keys=True),
    )
    assert "@article{doe2024fast," in generated.bibtex

    duplicate = rk.tidy_bibtex(
        """
@article{first, title={Same}, doi={10.1/example}, year={2024}}
@article{second, title={Same}, doi={10.1/example}, year={2025}}
""",
        options=rk.TidyOptions(duplicates=["doi"], merge="first"),
    )
    assert [warning.rule for warning in duplicate.warnings] == ["doi"]
    assert duplicate.count == 2
    assert duplicate.bibtex.count("@article") == 1
    assert duplicate.bibtex == (
        "@article{first,\n"
        "  title         = {Same},\n"
        "  doi           = {10.1/example},\n"
        "  year          = {2024}\n"
        "}\n"
    )


def test_tidy_options_constructor_rejects_positional_arguments() -> None:
    options_type = cast(Any, rk.TidyOptions)
    with pytest.raises(TypeError):
        options_type(True)
