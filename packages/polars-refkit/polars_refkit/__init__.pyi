from collections.abc import Iterable
from typing import Literal, TypeAlias

import polars as pl

__version__: str

RecoveryMode: TypeAlias = Literal["error", "report"]
DuplicateRule: TypeAlias = Literal["doi", "key", "abstract", "citation"]
MergeStrategy: TypeAlias = Literal["first", "last", "combine", "overwrite"]
ColumnExpr: TypeAlias = str | pl.Expr
TidyStringList: TypeAlias = Iterable[str] | None
TidyDuplicateRules: TypeAlias = Iterable[DuplicateRule] | None
TidyDefaultableUsize: TypeAlias = bool | int | None
TidyDefaultableString: TypeAlias = bool | str | None
TidyDefaultableStringList: TypeAlias = bool | Iterable[str] | None

class RefkitExprNamespace:
    def cite(
        self,
        key_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr: ...
    def cite_html(
        self,
        key_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr: ...
    def cite_rendered(
        self,
        key_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr: ...
    def cite_each(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr: ...
    def cite_each_html(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr: ...
    def cite_each_rendered(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr: ...
    def cite_group(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr: ...
    def cite_group_html(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr: ...
    def cite_group_rendered(
        self,
        keys_col: ColumnExpr,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr: ...
    def full_bibliography_html(
        self,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr: ...
    def full_bibliography_text(
        self,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr: ...
    def full_bibliography_rendered(
        self,
        *,
        style: str = "apa",
        locale: str = "en-US",
        recovery: RecoveryMode = "error",
    ) -> pl.Expr: ...
    def entry_count(self, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
    def can_parse(self, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
    def has_diagnostics(self, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
    def keys(self, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
    def entries(
        self,
        *,
        fields: Iterable[str] | None = None,
        recovery: RecoveryMode = "error",
    ) -> pl.Expr: ...
    def diagnostics(self, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
    def parse_report(self, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
    def to_hayagriva_json(self, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
    def tidy_bibtex(
        self,
        *,
        omit: TidyStringList = ...,
        curly: bool = ...,
        numeric: bool = ...,
        months: bool = ...,
        space: int = ...,
        tab: bool = ...,
        align: TidyDefaultableUsize = ...,
        blank_lines: bool = ...,
        sort: TidyDefaultableStringList = ...,
        duplicates: TidyDuplicateRules = ...,
        merge: MergeStrategy | None = ...,
        strip_enclosing_braces: bool = ...,
        drop_all_caps: bool = ...,
        escape: bool = ...,
        sort_fields: TidyDefaultableStringList = ...,
        strip_comments: bool = ...,
        trailing_commas: bool = ...,
        encode_urls: bool = ...,
        tidy_comments: bool = ...,
        remove_empty_fields: bool = ...,
        remove_duplicate_fields: bool = ...,
        generate_keys: TidyDefaultableString = ...,
        max_authors: int | None = ...,
        lowercase: bool = ...,
        enclosing_braces: TidyDefaultableStringList = ...,
        remove_braces: TidyDefaultableStringList = ...,
        wrap: TidyDefaultableUsize = ...,
    ) -> pl.Expr: ...
    def tidy_bibtex_report(
        self,
        *,
        omit: TidyStringList = ...,
        curly: bool = ...,
        numeric: bool = ...,
        months: bool = ...,
        space: int = ...,
        tab: bool = ...,
        align: TidyDefaultableUsize = ...,
        blank_lines: bool = ...,
        sort: TidyDefaultableStringList = ...,
        duplicates: TidyDuplicateRules = ...,
        merge: MergeStrategy | None = ...,
        strip_enclosing_braces: bool = ...,
        drop_all_caps: bool = ...,
        escape: bool = ...,
        sort_fields: TidyDefaultableStringList = ...,
        strip_comments: bool = ...,
        trailing_commas: bool = ...,
        encode_urls: bool = ...,
        tidy_comments: bool = ...,
        remove_empty_fields: bool = ...,
        remove_duplicate_fields: bool = ...,
        generate_keys: TidyDefaultableString = ...,
        max_authors: int | None = ...,
        lowercase: bool = ...,
        enclosing_braces: TidyDefaultableStringList = ...,
        remove_braces: TidyDefaultableStringList = ...,
        wrap: TidyDefaultableUsize = ...,
    ) -> pl.Expr: ...

def cite(
    bibtex_col: ColumnExpr,
    key_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr: ...
def cite_html(
    bibtex_col: ColumnExpr,
    key_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr: ...
def cite_rendered(
    bibtex_col: ColumnExpr,
    key_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr: ...
def cite_each(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr: ...
def cite_each_html(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr: ...
def cite_each_rendered(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr: ...
def cite_group(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr: ...
def cite_group_html(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr: ...
def cite_group_rendered(
    bibtex_col: ColumnExpr,
    keys_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr: ...
def full_bibliography_html(
    bibtex_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr: ...
def full_bibliography_text(
    bibtex_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr: ...
def full_bibliography_rendered(
    bibtex_col: ColumnExpr,
    *,
    style: str = "apa",
    locale: str = "en-US",
    recovery: RecoveryMode = "error",
) -> pl.Expr: ...
def entry_count(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
def can_parse(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
def has_diagnostics(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
def keys(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
def entries(
    bibtex_col: ColumnExpr,
    *,
    fields: Iterable[str] | None = None,
    recovery: RecoveryMode = "error",
) -> pl.Expr: ...
def diagnostics(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
def parse_report(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
def to_hayagriva_json(bibtex_col: ColumnExpr, *, recovery: RecoveryMode = "error") -> pl.Expr: ...
def tidy_bibtex(
    bibtex_col: ColumnExpr,
    *,
    omit: TidyStringList = ...,
    curly: bool = ...,
    numeric: bool = ...,
    months: bool = ...,
    space: int = ...,
    tab: bool = ...,
    align: TidyDefaultableUsize = ...,
    blank_lines: bool = ...,
    sort: TidyDefaultableStringList = ...,
    duplicates: TidyDuplicateRules = ...,
    merge: MergeStrategy | None = ...,
    strip_enclosing_braces: bool = ...,
    drop_all_caps: bool = ...,
    escape: bool = ...,
    sort_fields: TidyDefaultableStringList = ...,
    strip_comments: bool = ...,
    trailing_commas: bool = ...,
    encode_urls: bool = ...,
    tidy_comments: bool = ...,
    remove_empty_fields: bool = ...,
    remove_duplicate_fields: bool = ...,
    generate_keys: TidyDefaultableString = ...,
    max_authors: int | None = ...,
    lowercase: bool = ...,
    enclosing_braces: TidyDefaultableStringList = ...,
    remove_braces: TidyDefaultableStringList = ...,
    wrap: TidyDefaultableUsize = ...,
) -> pl.Expr: ...
def tidy_bibtex_report(
    bibtex_col: ColumnExpr,
    *,
    omit: TidyStringList = ...,
    curly: bool = ...,
    numeric: bool = ...,
    months: bool = ...,
    space: int = ...,
    tab: bool = ...,
    align: TidyDefaultableUsize = ...,
    blank_lines: bool = ...,
    sort: TidyDefaultableStringList = ...,
    duplicates: TidyDuplicateRules = ...,
    merge: MergeStrategy | None = ...,
    strip_enclosing_braces: bool = ...,
    drop_all_caps: bool = ...,
    escape: bool = ...,
    sort_fields: TidyDefaultableStringList = ...,
    strip_comments: bool = ...,
    trailing_commas: bool = ...,
    encode_urls: bool = ...,
    tidy_comments: bool = ...,
    remove_empty_fields: bool = ...,
    remove_duplicate_fields: bool = ...,
    generate_keys: TidyDefaultableString = ...,
    max_authors: int | None = ...,
    lowercase: bool = ...,
    enclosing_braces: TidyDefaultableStringList = ...,
    remove_braces: TidyDefaultableStringList = ...,
    wrap: TidyDefaultableUsize = ...,
) -> pl.Expr: ...
