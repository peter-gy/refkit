# Repository Contracts

Scripts under this directory validate source, generated state, wheels, sdists, and release metadata.

- Keep checks deterministic and free from network access.
- Accept an explicit root or artifact path when tests need an isolated fixture.
- Aggregate violations and name the offending file, field, or archive member.
- Send contract failures to stderr or raise a clear `SystemExit`. Keep successful machine-readable output stable.
- Add focused tests for every new accepted state and failure mode.
- Wire a durable source check into a Make target and the reusable source-check workflow.

The architecture contract classifies top-level, target-specific, build, and renamed Rust dependencies. It rejects unclassified Rust dependencies, host I/O in the portable core, direct engine dependencies in adapters, unclassified `refkit` runtime dependencies, and workspace changes that alter composition ownership.

Direct Maturin builds call `agent-plugins attach-wheel` before archive validation. The distribution contract owns each adapter's plugin, skill, entry-point, backend, and dependency artifact shape and compares skill bytes with release sources. `artifact_manifest.py` records build provenance and verifies the complete package artifact set before publication.

The core uses released BibLaTeX and audited Hayagriva and Citationberg revisions declared by the architecture contract. Keep canonical repositories, full commit revisions, manifest versions, Citationberg workspace patches, and Cargo locks aligned. The core pins quick-xml 0.41.0, and every resolved quick-xml version must meet that floor.

Run `make python-lint typecheck test`. See [repository contracts](../development_docs/repository-contracts.md) for ownership across scripts.

Benchmark CI orchestration keeps both revisions on one runner and uses the candidate harness and Python dependency lock. Fingerprint tracked runtime, build, lock, and harness inputs before starting measurement jobs. Reuse complete matching evidence while preserving its measured commits and environment. Consolidated reports preserve failed platforms and raw samples. Release attachments verify the requested commit, fingerprint, and raw timing hashes. The commit publisher runs from the default branch, consumes validated artifact data, and checks main-branch run and commit identities before updating its bot comment. Keep artifact publication separate from executing candidate code.

Give each benchmark revision separate Cargo target and intermediate build directories, including when the caller supplies a shared cache root. Verify compiled source identity through observable behavior.

`release_recovery.py` validates retained tag-run identity, completed publication gates, and available artifacts. The Publish workflow owns GitHub API reads and passes JSON records to this deterministic check before npm recovery.
