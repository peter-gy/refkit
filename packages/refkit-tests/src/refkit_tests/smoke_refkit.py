from __future__ import annotations

import sys
from importlib.metadata import distribution

import refkit as rk
import refkit.agent as refkit_agent
from refkit_tests.agent_examples import run_examples


def main() -> None:
    run_examples("refkit")

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
    assert "refkit.Library" in refkit_agent.instructions()
    resources = refkit_agent.resources()
    assert set(resources) == {
        "SKILL.md",
        "agents/openai.yaml",
        "references/contracts.md",
        "references/inspect.md",
        "references/render.md",
        "references/edit.md",
        "references/tidy.md",
    }
    assert all(path.is_file() for path in resources.values())

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
    rendered = document.render([rk.Citation(id="intro", citation="doe2024")])

    assert "Doe" in rendered["intro"].text
    assert rendered.bibliography.text
    sys.stdout.write(f"{rk.build_info}\n")
    sys.stdout.write(f"{rendered['intro'].text}\n")


if __name__ == "__main__":
    main()
