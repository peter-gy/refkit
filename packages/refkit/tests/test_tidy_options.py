from __future__ import annotations

from typing import Any, cast

import pytest

import refkit as rk


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
    assert sorted_fields.bibtex.index("author") < sorted_fields.bibtex.index("year")

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


def test_complete_formatter_configuration_transforms_records() -> None:
    options = rk.TidyOptions(
        omit=["abstract"],
        curly=True,
        numeric=True,
        months=False,
        space=2,
        tab=False,
        align=14,
        blank_lines=True,
        sort=["key"],
        duplicates=["doi"],
        merge="first",
        strip_enclosing_braces=False,
        drop_all_caps=False,
        escape=True,
        sort_fields=["title", "author"],
        strip_comments=False,
        trailing_commas=False,
        encode_urls=False,
        tidy_comments=False,
        remove_empty_fields=True,
        remove_duplicate_fields=True,
        generate_keys="[auth:lower][year]",
        max_authors=3,
        lowercase=True,
        enclosing_braces=["title"],
        remove_braces=["note"],
        wrap=80,
    )
    source = "@article{old,author={Doe, Jane},title={Work},year=2024,abstract={Omitted}}"
    formatted = rk.tidy_bibtex(source, options=options)
    document = rk.BibDocument.parse(formatted.bibtex)
    assert document.entries.unique_keys() == ["doe2024"]
    assert document.entries["doe2024"].fields["title"].value == "{Work}"
    assert document.entries["doe2024"].fields.unique_keys() == ["title", "author", "year"]
