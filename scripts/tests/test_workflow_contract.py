from pathlib import Path

import yaml


def test_documentation_deployment_waits_for_required_checks() -> None:
    root = Path(__file__).resolve().parents[2]
    workflow = yaml.safe_load((root / ".github/workflows/ci.yml").read_text())
    jobs = workflow["jobs"]

    assert jobs["deploy-docs"]["needs"] == "check"
    assert {"source-checks", "docs"} <= set(jobs["check"]["needs"])
