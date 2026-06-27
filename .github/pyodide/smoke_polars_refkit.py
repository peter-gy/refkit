from __future__ import annotations

import sys
from typing import Any, cast

import polars as pl

import polars_refkit
import refkit as rk

BIBTEX = """
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  journal = {Journal of Citation Tests},
  year = {2024}
}
"""


def main() -> None:
    assert rk.check_refkit_core_version()
    assert rk.Library.parse_bibtex(BIBTEX)
    assert polars_refkit.__version__

    frame = pl.DataFrame(
        {
            "bibtex": [BIBTEX],
            "key": ["doe2024"],
            "keys": [["doe2024"]],
        }
    )
    namespace = cast(Any, pl.col("bibtex")).refkit
    row = frame.select(
        count=namespace.entry_count(),
        citation=namespace.cite("key", style="apa"),
        rendered=namespace.cite_rendered("key", style="apa"),
        each=namespace.cite_each("keys", style="apa"),
    ).to_dicts()[0]

    assert row["count"] == 1
    assert row["citation"] == "(Doe, 2024)"
    assert row["rendered"]["text"] == "(Doe, 2024)"
    assert row["rendered"]["html"]
    assert row["each"] == ["(Doe, 2024)"]

    sys.stdout.write(f"polars {pl.__version__}\n")
    sys.stdout.write(f"polars-refkit {polars_refkit.__version__}\n")
    sys.stdout.write(f"{row['citation']}\n")


if __name__ == "__main__":
    main()
