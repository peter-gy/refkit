# Benchmark Package

`refkit-bench` measures public capability workflows and verifies correctness before accepting timing rows.

- Keep this package outside the runtime dependency graph. Runtime packages never depend on benchmark code, fixtures, adapters, or result schemas.
- Exercise participants through their public package boundaries.
- Give each lane one workflow, source format, setup contract, measured phase, and fair participant set.
- Keep comparison-package behavior in adapters. Do not distort a package API to force it into an unrelated lane.
- Record workload provenance, source license, hashes, setup placement, execution mode, package versions, and build mode.
- Generate real BibTeX and CSL JSON from the same curated records. Verify complete author lists and every compared field through an independent parser.
- Record measured-artifact identity and observed build mode separately from the runner checkout and caller assertions. Use unknown when installed metadata cannot establish a source revision.
- Build native adapters in release mode before collecting timing evidence.
- Keep generated JSON and CSV under `results/` and out of commits. Track runner code, tests, and audited fixtures.
- Update result schemas, writers, tests, and the developer guide together.

Run `make benchmark-test`. See the [benchmark guide](../../development_docs/benchmarks.md) for commands and interpretation rules.
