"""Execute the examples shipped in an installed adapter's Agent Skill."""

from __future__ import annotations

import re
import sys

import agent_plugins


def run_examples(distribution_name: str) -> int:
    """Execute each Python block independently and return the example count."""
    plugin = agent_plugins.locate(distribution_name)
    skill = plugin.skill(distribution_name)
    count = 0
    for resource in skill.files:
        relative = resource.relative_to(skill.path)
        if relative.parts[0] != "references" or resource.suffix != ".md":
            continue
        source = resource.read_text(encoding="utf-8")
        for index, match in enumerate(
            re.finditer(r"^```python\n(.*?)^```\s*$", source, re.M | re.S)
        ):
            code = compile(match[1], f"{resource}:example-{index + 1}", "exec")
            exec(code, {"__name__": "__refkit_example__"})
            count += 1
    if count == 0:
        raise AssertionError(f"{distribution_name} ships no executable task examples")
    return count


if __name__ == "__main__":
    for distribution in sys.argv[1:]:
        sys.stdout.write(
            f"{distribution}: {run_examples(distribution)} installed examples passed\n"
        )
