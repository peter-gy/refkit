# Benchmarks

`refkit-bench` measures public bibliography workflows across RefKit interfaces and comparison packages. Each lane owns one capability, workload, setup contract, measured phase, and correctness check.

## Build For Measurements

Timing runs require release-mode native extensions:

```bash
uv sync --locked --all-packages --group dev
(cd packages/refkit-core && uv run maturin develop --release)
(cd packages/polars-refkit && uv run maturin develop --release)
```

Run `make benchmark-test` after changing lanes, fixtures, adapters, result fields, or output writers.

## Run Lanes

List available lanes:

```bash
uv run --locked --package refkit-bench python -m refkit_bench.runner --list
```

Run one lane:

```bash
uv run --locked --package refkit-bench python -m refkit_bench.runner \
  --lane input.bibtex \
  --rounds 3 \
  --warmups 1 \
  --json packages/refkit-bench/results/smoke.json
```

Run every group and write JSON plus CSV:

```bash
uv run --locked --package refkit-bench python -m refkit_bench.runner \
  --group all \
  --input all \
  --rounds 5 \
  --warmups 2 \
  --json packages/refkit-bench/results/latest.json \
  --csv packages/refkit-bench/results/latest.csv
```

Result files belong in `packages/refkit-bench/results/` and remain local. Commit benchmark code and audited fixtures.

## Lane Contract

Compare rows only when `lane`, `input_size`, `source_format`, `setup_included`, and `execution_mode` describe the same workflow.

| Field group | Meaning |
| --- | --- |
| `lane`, `group`, `capability`, `workflow` | Public workflow under measurement. |
| `package`, `package_version`, `adapter_version` | Implementation under test. |
| `phase`, `operation_phase` | Lane-owned measured phase. |
| Input identity and hashes | Workload provenance, size, format, and fingerprint. |
| `execution_mode` | Eager or lazy Polars execution when relevant. |
| `setup_included`, `setup_seconds` | Placement and cost of setup work. |
| `rounds`, `warmups`, `round`, `seconds` | Measurement configuration and result. |
| `status`, `operation_count` | Correctness outcome and work completed. |
| Runtime metadata | Python, operating system, CPU, RefKit version, commit, and build mode. |

`ok` means execution and the lane correctness check passed. `failed` records setup, execution, or correctness failure. `unsupported` means the adapter declared the workflow outside its public contract before timing.

## Capability Phases

| Capability | Measured boundaries to isolate |
| --- | --- |
| Normalized input | File read, decode, recovery, BibTeX parse, normalization, and host materialization. |
| Raw BibTeX | Block scan, occurrence indexes, field validation, patching, and writeback. |
| Style input | Archive lookup, XML parse, locale lookup, and processor construction. |
| Citation rendering | Cite parsing, key lookup, locator validation, driver work, disambiguation, and ordered per-call history. |
| Bibliography rendering | Citation collection, sorting, disambiguation, output creation, and tree materialization. |
| Rendered output | Cached lookup, HTML generation, and host tree conversion. |
| Entry inspection | Selector work, projection, cache lookup, and host row creation. |
| Polars bulk work | Row parsing, capability execution, Arrow conversion, and eager or lazy plan execution. |

Use `setup_seconds` before attributing time to a measured operation. A prepared render lane excludes source and style setup. A path-based one-call lane includes file read, parse, style load, processor setup, and render. Polars lanes parse and execute inside the dataframe operation.

## Inputs

The runner generates deterministic `tiny`, `medium`, and `large` workloads. The `real` workload uses [`references.bib`](../packages/refkit-bench/src/refkit_bench/data/real-bibliography/references.bib) and its adjacent [provenance note](../packages/refkit-bench/src/refkit_bench/data/real-bibliography/README.md).

Generated workloads provide clean BibTeX, raw BibTeX with top-level blocks, malformed BibTeX, duplicate entries or fields, and CSL JSON for comparison renderers. The real workload uses the same clean bibliography for its clean, raw, and dirty source fields, so it provides syntax diversity rather than malformed-input recovery evidence. A lane selects the source form that matches its public workflow.

## Comparison Rules

- Schedule a package only when its public API owns the workflow.
- Keep setup placement equal inside one lane and record setup performed outside the measured rounds.
- Run a correctness check before accepting timing output.
- Preserve input identity, source license, record count, byte count, and hash in result rows.
- Use release-mode native builds for timing claims.
- Report failed and unsupported rows separately from successful timing comparisons.
- Back every performance claim with the command, inputs, runtime metadata, and result artifact that produced it.

The current comparison adapters cover `bibtexparser`, `citeproc-py`, and Pybtex where their public workflows overlap a RefKit lane. The [feature matrix](feature-matrix.md) records the broader inspected capability context.
