"""Polars expressions for row-level BibTeX citation workflows."""

from importlib.metadata import version as _metadata_version

from ._expressions import can_parse as can_parse
from ._expressions import cite as cite
from ._expressions import cite_each as cite_each
from ._expressions import cite_group as cite_group
from ._expressions import diagnostics as diagnostics
from ._expressions import entries as entries
from ._expressions import entry_count as entry_count
from ._expressions import full_bibliography as full_bibliography
from ._expressions import has_diagnostics as has_diagnostics
from ._expressions import keys as keys
from ._expressions import parse_report as parse_report
from ._expressions import render_report as render_report
from ._expressions import tidy_bibtex as tidy_bibtex
from ._expressions import tidy_bibtex_report as tidy_bibtex_report
from ._internal import build_mode as build_mode
from ._namespace import RefkitExprNamespace as RefkitExprNamespace
from ._plugin import ColumnExpr as ColumnExpr
from ._plugin import OutputFormat as OutputFormat
from ._plugin import RecoveryMode as RecoveryMode
from ._tidy_options import DuplicateRule as DuplicateRule
from ._tidy_options import MergeStrategy as MergeStrategy
from ._tidy_options import TidyOptions as TidyOptions

__version__ = _metadata_version("polars-refkit")

__all__ = [
    "__version__",
    "build_mode",
    "RefkitExprNamespace",
    "TidyOptions",
    "OutputFormat",
    "ColumnExpr",
    "RecoveryMode",
    "DuplicateRule",
    "MergeStrategy",
    "cite",
    "cite_each",
    "cite_group",
    "full_bibliography",
    "render_report",
    "entry_count",
    "can_parse",
    "has_diagnostics",
    "keys",
    "entries",
    "diagnostics",
    "parse_report",
    "tidy_bibtex",
    "tidy_bibtex_report",
]
