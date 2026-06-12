from __future__ import annotations

from benchmark._adapters.bibtexparser_v2 import (
    BibtexparserV2Adapter,
    _bibtexparser_block_key,
    _bibtexparser_v2_failed_signatures,
    _bibtexparser_v2_field_value,
    _bibtexparser_v2_recovery_matches,
    _bibtexparser_v2_set_field,
)
from benchmark._adapters.citeproc_py import CiteprocPyAdapter
from benchmark._adapters.common import (
    OperationOutcome,
    PackageAdapter,
    PreparedOperation,
    UnsupportedOperation,
    _all_checks,
    _bibliography_output_matches,
    _citation_output_matches,
    _count_at_least,
    _count_is,
    _detail_contains,
    _entries_match,
    _error_detail,
    _first,
    _keys_are,
    _lookup_keys,
    _prepared,
    _projection_contains,
    _raw_roundtrip_check,
    _recovery_parse_result,
    _text_contains,
    _text_contains_all,
)
from benchmark._adapters.polars_refkit import PolarsRefkitAdapter
from benchmark._adapters.refkit import RefkitAdapter


def adapters() -> list[PackageAdapter]:
    return [
        RefkitAdapter(),
        PolarsRefkitAdapter(),
        CiteprocPyAdapter(),
        BibtexparserV2Adapter(),
    ]


__all__ = [
    "BibtexparserV2Adapter",
    "CiteprocPyAdapter",
    "OperationOutcome",
    "PackageAdapter",
    "PolarsRefkitAdapter",
    "PreparedOperation",
    "RefkitAdapter",
    "UnsupportedOperation",
    "_all_checks",
    "_bibliography_output_matches",
    "_bibtexparser_block_key",
    "_bibtexparser_v2_failed_signatures",
    "_bibtexparser_v2_field_value",
    "_bibtexparser_v2_recovery_matches",
    "_bibtexparser_v2_set_field",
    "_citation_output_matches",
    "_count_at_least",
    "_count_is",
    "_detail_contains",
    "_entries_match",
    "_error_detail",
    "_first",
    "_keys_are",
    "_lookup_keys",
    "_prepared",
    "_projection_contains",
    "_raw_roundtrip_check",
    "_recovery_parse_result",
    "_text_contains",
    "_text_contains_all",
    "adapters",
]
