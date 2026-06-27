from __future__ import annotations

from refkit_bench._adapters.bibtexparser_v2 import (
    BibtexparserV2Adapter,
    _bibtexparser_v2_failed_signatures,
)
from refkit_bench._adapters.citeproc_py import CiteprocPyAdapter
from refkit_bench._adapters.common import (
    MissingBenchmarkOperation,
    OperationOutcome,
    PackageAdapter,
    PreparedOperation,
    _all_checks,
    _bibliography_output_matches,
    _citation_output_matches,
    _count_at_least,
    _count_is,
    _detail_contains,
    _duplicate_signals_cover,
    _entries_match,
    _first,
    _keys_are,
    _lookup_keys,
    _prepared,
    _projection_contains,
    _raw_blocks_cover,
    _raw_roundtrip_check,
    _text_contains,
)
from refkit_bench._adapters.polars_refkit import PolarsRefkitAdapter
from refkit_bench._adapters.pybtex import PybtexAdapter
from refkit_bench._adapters.refkit import RefkitAdapter


def adapters() -> list[PackageAdapter]:
    return [
        RefkitAdapter(),
        PolarsRefkitAdapter(),
        CiteprocPyAdapter(),
        BibtexparserV2Adapter(),
        PybtexAdapter(),
    ]


__all__ = [
    "BibtexparserV2Adapter",
    "CiteprocPyAdapter",
    "MissingBenchmarkOperation",
    "OperationOutcome",
    "PackageAdapter",
    "PolarsRefkitAdapter",
    "PreparedOperation",
    "PybtexAdapter",
    "RefkitAdapter",
    "_all_checks",
    "_bibliography_output_matches",
    "_bibtexparser_v2_failed_signatures",
    "_citation_output_matches",
    "_count_at_least",
    "_count_is",
    "_detail_contains",
    "_duplicate_signals_cover",
    "_entries_match",
    "_first",
    "_keys_are",
    "_lookup_keys",
    "_prepared",
    "_projection_contains",
    "_raw_blocks_cover",
    "_raw_roundtrip_check",
    "_text_contains",
    "adapters",
]
