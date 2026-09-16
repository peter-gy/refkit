from pathlib import Path

import yaml


def test_documentation_deployment_waits_for_required_checks() -> None:
    root = Path(__file__).resolve().parents[2]
    workflow = yaml.safe_load((root / ".github/workflows/ci.yml").read_text())
    jobs = workflow["jobs"]

    assert jobs["deploy-docs"]["needs"] == "check"
    assert {"source-checks", "docs"} <= set(jobs["check"]["needs"])


def test_comment_publisher_executes_default_branch_code_with_scoped_permissions() -> None:
    root = Path(__file__).resolve().parents[2]
    workflow = yaml.safe_load((root / ".github/workflows/benchmark-comment.yml").read_text())
    assert workflow["on"]["workflow_run"] == {
        "workflows": ["Benchmarks"],
        "types": ["completed"],
    }
    job = workflow["jobs"]["comment"]
    assert job["permissions"] == {"contents": "write", "actions": "read"}
    checkout = job["steps"][0]
    assert checkout["with"]["ref"] == "${{ github.event.repository.default_branch }}"
    assert checkout["with"]["persist-credentials"] is False


def test_npm_publish_uses_tested_artifact_and_trusted_environment() -> None:
    root = Path(__file__).resolve().parents[2]
    jobs = yaml.safe_load((root / ".github/workflows/publish.yml").read_text())["jobs"]
    publish = jobs["publish-refkit-js"]
    assert publish["environment"] == "npm"
    assert publish["permissions"]["id-token"] == "write"
    assert {"source-checks", "artifacts-refkit-js", "check-release-version"} <= set(
        publish["needs"]
    )
    assert "publish-refkit-js" in jobs["release-complete"]["needs"]
    download = next(
        step for step in publish["steps"] if "download-artifact@" in step.get("uses", "")
    )
    assert download["with"]["name"] == "refkit-js-npm"
    command = next(step["run"] for step in publish["steps"] if "npm publish" in step.get("run", ""))
    assert '"./dist/refkit-js-${RELEASE_VERSION}.tgz"' in command
    assert "--provenance" in command
    ci = yaml.safe_load((root / ".github/workflows/ci.yml").read_text())["jobs"]
    assert "artifacts-refkit-js" in ci["check"]["needs"]
