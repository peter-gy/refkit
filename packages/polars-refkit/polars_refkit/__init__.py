"""Polars expressions for row-level BibTeX citation workflows."""

from __future__ import annotations

from importlib.metadata import version as _metadata_version
from typing import Any

from . import _tidy_options
from ._expressions import can_parse as can_parse
from ._expressions import cite as cite
from ._expressions import cite_each as cite_each
from ._expressions import cite_each_html as cite_each_html
from ._expressions import cite_each_rendered as cite_each_rendered
from ._expressions import cite_group as cite_group
from ._expressions import cite_group_html as cite_group_html
from ._expressions import cite_group_rendered as cite_group_rendered
from ._expressions import cite_html as cite_html
from ._expressions import cite_rendered as cite_rendered
from ._expressions import diagnostics as diagnostics
from ._expressions import entries as entries
from ._expressions import entry_count as entry_count
from ._expressions import full_bibliography_html as full_bibliography_html
from ._expressions import full_bibliography_rendered as full_bibliography_rendered
from ._expressions import full_bibliography_text as full_bibliography_text
from ._expressions import has_diagnostics as has_diagnostics
from ._expressions import keys as keys
from ._expressions import parse_report as parse_report
from ._expressions import tidy_bibtex as tidy_bibtex
from ._expressions import tidy_bibtex_report as tidy_bibtex_report
from ._expressions import to_hayagriva_json as to_hayagriva_json
from ._namespace import RefkitExprNamespace as RefkitExprNamespace
from ._plugin import PLUGIN_PATH as PLUGIN_PATH
from ._plugin import ColumnExpr as ColumnExpr
from ._plugin import RecoveryMode as RecoveryMode
from ._tidy_options import DuplicateRule as DuplicateRule
from ._tidy_options import MergeStrategy as MergeStrategy

_TIDY_UNSET = _tidy_options.TIDY_UNSET
_tidy_kwargs = _tidy_options.tidy_kwargs
_tidy_signature = _tidy_options.tidy_signature

__version__ = _metadata_version("polars-refkit")

__all__ = (
    "__version__ DuplicateRule MergeStrategy RefkitExprNamespace "
    "can_parse cite cite_html cite_rendered cite_each cite_each_html "
    "cite_each_rendered cite_group cite_group_html cite_group_rendered "
    "diagnostics entries entry_count has_diagnostics keys parse_report "
    "to_hayagriva_json full_bibliography_html full_bibliography_rendered "
    "full_bibliography_text tidy_bibtex tidy_bibtex_report"
).split()


def _set_signature(target: Any, signature: object) -> None:
    target.__signature__ = signature


for _target, _first_parameter in (
    (tidy_bibtex, "bibtex_col"),
    (tidy_bibtex_report, "bibtex_col"),
    (RefkitExprNamespace.tidy_bibtex, "self"),
    (RefkitExprNamespace.tidy_bibtex_report, "self"),
):
    _set_signature(_target, _tidy_signature(_first_parameter))
