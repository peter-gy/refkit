from __future__ import annotations

from typing import Any, Literal, TypeAlias, TypedDict, cast

DuplicateRule: TypeAlias = Literal["doi", "key", "abstract", "citation"]
MergeStrategy: TypeAlias = Literal["first", "last", "combine", "overwrite"]


_TIDY_BOOL_OPTIONS = {
    "curly",
    "numeric",
    "months",
    "tab",
    "blank_lines",
    "strip_enclosing_braces",
    "drop_all_caps",
    "escape",
    "strip_comments",
    "trailing_commas",
    "encode_urls",
    "tidy_comments",
    "remove_empty_fields",
    "remove_duplicate_fields",
    "lowercase",
}
_TIDY_STRING_LIST_OPTIONS = {"omit", "duplicates"}
_TIDY_DEFAULTABLE_STRING_LIST_OPTIONS = {
    "sort",
    "sort_fields",
    "enclosing_braces",
    "remove_braces",
}
_TIDY_DEFAULTABLE_USIZE_OPTIONS = {"align", "wrap"}
_VALID_DUPLICATE_RULES = {"doi", "key", "abstract", "citation"}
_VALID_MERGE_STRATEGIES = {"first", "last", "combine", "overwrite"}


class TidyOptions(TypedDict, total=False):
    """Serializable formatter options. Omitted fields use core defaults."""

    omit: list[str] | None
    curly: bool
    numeric: bool
    months: bool
    space: int
    tab: bool
    align: bool | int | None
    blank_lines: bool
    sort: bool | list[str] | None
    duplicates: list[DuplicateRule] | None
    merge: MergeStrategy | None
    strip_enclosing_braces: bool
    drop_all_caps: bool
    escape: bool
    sort_fields: bool | list[str] | None
    strip_comments: bool
    trailing_commas: bool
    encode_urls: bool
    tidy_comments: bool
    remove_empty_fields: bool
    remove_duplicate_fields: bool
    generate_keys: bool | str | None
    max_authors: int | None
    lowercase: bool
    enclosing_braces: bool | list[str] | None
    remove_braces: bool | list[str] | None
    wrap: bool | int | None


def tidy_kwargs(options: TidyOptions | None) -> dict[str, Any]:
    if options is not None and not isinstance(options, dict):
        raise TypeError("options must be a dictionary")
    kwargs: dict[str, Any] = {}
    for name, value in (options or {}).items():
        if name in _TIDY_BOOL_OPTIONS:
            kwargs[name] = _bool_option(name, value)
        elif name == "space":
            kwargs[name] = _usize_option(name, value)
        elif name in _TIDY_DEFAULTABLE_USIZE_OPTIONS:
            kwargs[name] = _defaultable_usize_option(name, value)
        elif name in _TIDY_STRING_LIST_OPTIONS:
            if name == "duplicates" and value is None:
                continue
            kwargs[name] = (
                _duplicate_rules_option(value)
                if name == "duplicates"
                else _string_list_option(name, value)
            )
        elif name in _TIDY_DEFAULTABLE_STRING_LIST_OPTIONS:
            kwargs[name] = _defaultable_string_list_option(name, value)
        elif name == "generate_keys":
            kwargs[name] = _defaultable_string_option(name, value)
        elif name == "merge":
            if value is not None:
                kwargs[name] = _merge_strategy_option(value)
        elif name == "max_authors":
            if value is not None:
                kwargs[name] = _usize_option(name, value)
        else:
            raise ValueError(f"unknown tidy option {name!r}")
    return kwargs


def _bool_option(name: str, value: Any) -> bool:
    if isinstance(value, bool):
        return value
    raise TypeError(f"{name} must be a bool")


def _usize_option(name: str, value: Any) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise TypeError(f"{name} must be an integer")
    if value < 0:
        raise ValueError(f"{name} must be non-negative")
    return value


def _string_option(name: str, value: Any) -> str:
    if isinstance(value, str):
        return value
    raise TypeError(f"{name} must be a string")


def _string_list_option(name: str, value: Any) -> list[str]:
    if value is None:
        return []
    if isinstance(value, str):
        raise TypeError(f"{name} must be an iterable of strings")
    try:
        values = list(value)
    except TypeError as exc:
        raise TypeError(f"{name} must be an iterable of strings") from exc
    if not all(isinstance(item, str) for item in values):
        raise TypeError(f"{name} must be an iterable of strings")
    return values


def _duplicate_rules_option(value: Any) -> list[DuplicateRule]:
    values = _string_list_option("duplicates", value)
    for item in values:
        if item not in _VALID_DUPLICATE_RULES:
            raise ValueError(f"unknown duplicate rule {item!r}")
    return cast(list[DuplicateRule], values)


def _merge_strategy_option(value: Any) -> MergeStrategy:
    value = _string_option("merge", value)
    if value not in _VALID_MERGE_STRATEGIES:
        raise ValueError(f"unknown merge strategy {value!r}")
    return cast(MergeStrategy, value)


def _defaultable_usize_option(name: str, value: Any) -> bool | int:
    if value is None:
        return False
    if isinstance(value, bool):
        return value
    return _usize_option(name, value)


def _defaultable_string_option(name: str, value: Any) -> bool | str:
    if value is None:
        return False
    if isinstance(value, bool):
        return value
    return _string_option(name, value)


def _defaultable_string_list_option(name: str, value: Any) -> bool | list[str]:
    if value is None:
        return False
    if isinstance(value, bool):
        return value
    return _string_list_option(name, value)
