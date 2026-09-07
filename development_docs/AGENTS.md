# Developer Documentation

`development_docs/` explains cross-package architecture, capabilities, executable contracts, testing, documentation delivery, packaging, releases, benchmarks, and troubleshooting.

- Update `architecture.md` when dependency direction, capability ownership, adapter placement, or composition changes.
- Update `repository-contracts.md` with every executable invariant and its command.
- Update `documentation.md` when the VitePress source, navigation, public assets, validation, or delivery path changes.
- Ground package and workflow claims in current manifests, source, scripts, and CI.
- Keep package-local implementation instructions in the nearest `AGENTS.md`.
- Keep authored first-success examples executable through `make docs-examples-check` after native adapters are installed. Static site builds stay independent of native imports.
- Keep session plans and completion evidence out of tracked guidance.

Run `make docs-check test` from the repository root.
