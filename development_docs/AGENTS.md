# Developer Documentation

`development_docs/` explains cross-package architecture, executable contracts, testing, packaging, releases, and benchmarks.

- Update `architecture.md` when dependency direction, capability ownership, adapter placement, or composition changes.
- Update `repository-contracts.md` with every executable invariant and its command.
- Ground package and workflow claims in current manifests, source, scripts, and CI.
- Keep package-local implementation instructions in the nearest `AGENTS.md`.
- Keep session plans and completion evidence out of tracked guidance.

Run `make docs-check test` from the repository root.
