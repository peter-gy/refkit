from __future__ import annotations

import sys
from importlib.metadata import distribution

import refkit as rk
import refkit.agent as refkit_agent


def main() -> None:
    assert rk.build_info

    capabilities = [
        entry_point
        for entry_point in distribution("refkit").entry_points
        if entry_point.group == "marimo.agent.capability"
    ]
    assert [(entry.name, entry.value) for entry in capabilities] == [("refkit", "refkit.agent")]
    assert capabilities[0].load() is refkit_agent
    assert refkit_agent.agent_plugin().manifest.name == "refkit"
    assert refkit_agent.agent_skill().path.name == "refkit"

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
