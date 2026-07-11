# Repository Contracts

Scripts under this directory validate source, generated state, wheels, sdists, and release metadata.

- Keep checks deterministic and free from network access.
- Accept an explicit root or artifact path when tests need an isolated fixture.
- Aggregate violations and name the offending file, field, or archive member.
- Send contract failures to stderr or raise a clear `SystemExit`. Keep successful machine-readable output stable.
- Add focused tests for every new accepted state and failure mode.
- Wire a durable source check into a Make target and the reusable source-check workflow.

Run `make python-lint typecheck test`. See [repository contracts](../development_docs/repository-contracts.md) for ownership across scripts.
