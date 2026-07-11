# Benchmark Package

`refkit-bench` measures public capability workflows and verifies correctness before accepting timing rows.

- Give each lane one workflow, source format, setup contract, measured phase, and fair participant set.
- Keep comparison-package behavior in adapters. Do not distort a package API to force it into an unrelated lane.
- Record workload provenance, source license, hashes, setup placement, execution mode, package versions, and build mode.
- Build native adapters in release mode before collecting timing evidence.
- Keep generated JSON and CSV under `results/` and out of commits. Track runner code, tests, and audited fixtures.
- Update result schemas, writers, tests, and the developer guide together.

Run `make benchmark-test`. See the [benchmark guide](../../development_docs/benchmarks.md) for commands and interpretation rules.
