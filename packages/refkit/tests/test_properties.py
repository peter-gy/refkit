from __future__ import annotations

import string

import hypothesis.strategies as st
import pytest
from hypothesis import given, settings

import refkit as rk

KEYS = st.from_regex(r"[A-Za-z][A-Za-z0-9_-]{0,15}", fullmatch=True)
TEXT = st.text(alphabet=string.ascii_letters + string.digits + " ", min_size=1, max_size=32)
YEARS = st.integers(min_value=1000, max_value=9999)


def _entry(key: str, title: str, year: int) -> str:
    return f"@article{{{key}, title={{{title}}}, year={{{year}}}}}"


@given(key=KEYS, title=TEXT, year=YEARS)
@settings(max_examples=75, deadline=None)
def test_tidy_bibtex_is_idempotent_for_valid_entries(key: str, title: str, year: int) -> None:
    first = rk.tidy_bibtex(_entry(key, title, year)).bibtex
    second = rk.tidy_bibtex(first).bibtex

    assert second == first
    assert rk.Library.parse_bibtex(first).keys() == [key]


@given(key=KEYS, title=TEXT, replacement=TEXT, year=YEARS)
@settings(max_examples=50, deadline=None)
def test_raw_title_edits_survive_serialization(
    key: str,
    title: str,
    replacement: str,
    year: int,
) -> None:
    document = rk.BibDocument.parse(_entry(key, title, year))
    document.entries[key].fields["title"].value = replacement
    serialized = document.to_bibtex()

    assert replacement in serialized
    assert rk.Library.parse_bibtex(serialized).keys() == [key]


@given(key=KEYS, title=TEXT)
@settings(max_examples=50, deadline=None)
def test_default_parser_rejects_unclosed_entries(key: str, title: str) -> None:
    with pytest.raises(rk.RefkitError, match="parse error"):
        rk.Library.parse_bibtex(f"@article{{{key}, title={{{title}")
