# Benchmark Package

`refkit-bench` validates public capability contracts before accepting calibrated worker measurements.

- Keep this package outside the runtime dependency graph. Runtime packages never depend on benchmark code, fixtures, adapters, or result schemas.
- Exercise participants through their public package boundaries.
- Give each lane one complete operation, output contract, declared input representation, setup boundary, and eligible participant set.
- Keep comparison-package behavior in adapters. Do not distort a package API to force it into an unrelated lane.
- Record workload provenance, source license, hashes, setup placement, execution mode, package versions, and build mode.
- Generate real BibTeX and CSL JSON from the same curated records. Check every required field against independently authored expectations through public observable values.
- Fingerprint Python wrappers, native binaries, and relevant runtime dependencies. Keep artifact identity separate from the benchmark checkout. Use unknown when installed metadata cannot establish a source revision.
- Build native adapters in release mode before collecting timing evidence.
- Keep generated result directories under `results/` and out of commits. Track benchmark code, tests, audited fixtures, and dependency locks.
- Update result schemas, writers, tests, and the developer guide together.
- Validate every selected case before timing and the final result after each timed batch. Failed cases stay visible in conformance reports and prevent a selected measurement run from succeeding.
- Keep setup, validation, Node startup, and transport outside the declared clock. Each operation must handle its own mutable state consistently across repeated calls.
- Keep measurement workers in the preflight environment, including Python runtime flags. Verify measured worker contracts and artifacts against preflight. Fix Polars thread counts before importing the plugin in every worker.
- Apply CPU affinity before participant setup and child startup. Verify effective process and Linux thread masks outside the timing boundary.
- On Windows, enforce measurement deadlines through the parent case process. pyperf 2.10 uses a socket-only `select()` call for timed pipe reads.
- Compare independent worker observations with matched case and host contracts. Keep smoke runs distinct from measurements and report per-case uncertainty.
- Use the package-local coverage configuration for subprocess tests. Node dependencies belong under `node/` and are installed by `make benchmark-test`.

Run `make benchmark-test`. See the [benchmark guide](../../development_docs/benchmarks.md) for commands and interpretation rules.
