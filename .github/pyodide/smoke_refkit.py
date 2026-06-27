from __future__ import annotations

import sys

import refkit as rk


def main() -> None:
    assert rk.check_refkit_core_version()
    assert rk.build_info

    library = rk.Library.parse_bibtex(
        """
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  journal = {Journal of Citation Tests},
  year = {2024}
}
"""
    )
    document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
    rendered = document.render([rk.Citation("intro", "doe2024")])

    assert "Doe" in rendered["intro"].text
    assert rendered.bibliography.text
    sys.stdout.write(f"{rk.build_info}\n")
    sys.stdout.write(f"{rendered['intro'].text}\n")


if __name__ == "__main__":
    main()
