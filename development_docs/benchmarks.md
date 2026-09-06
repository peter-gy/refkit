# Benchmarks

`refkit-bench` measures public bibliography workflows across RefKit interfaces and comparison packages. Each lane owns one capability, workload, setup contract, measured phase, and correctness check.

## Build For Measurements

Timing runs require release-mode native extensions:

```bash
uv sync --locked --all-packages --group dev
(cd packages/refkit && uv run maturin develop --release)
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

## Command Selection

The runner defaults to all four inputs, five measured rounds, and two warmups. Repeated `--input` values are deduplicated. Repeated `--lane` values schedule repeated lane runs. Explicit lanes take precedence over `--group`.

The nine groups contain 32 lanes:

| Group | Lanes |
| --- | --- |
| `input.normalized` | `input.bibtex-text`, `input.bibtex`, `input.dirty-bibtex`, `input.diagnostics` |
| `raw-bibtex` | `raw-bibtex.parse`, `raw-bibtex.blocks`, `raw-bibtex.duplicates`, `raw-bibtex.write`, `raw-bibtex.roundtrip` |
| `style` | `style.load`, `style.processor-setup` |
| `render.prepared` | `render.prepared-citation`, `render.prepared-bibliography`, `render.cited-bibliography`, `render.repeated-citations` |
| `render.output` | `render.output-text`, `render.output-html`, `render.output-tree` |
| `render.one-off` | `render.one-off-cite`, `render.one-off-bibliography` |
| `errors` | `errors.missing-reference` |
| `inspect.entries` | `inspect.materialize`, `inspect.keys`, `inspect.lookup`, `inspect.fields` |
| `bulk.polars` | `bulk.polars.materialize`, `bulk.polars.keys`, `bulk.polars.lookup`, `bulk.polars.fields`, `bulk.polars.citation`, `bulk.polars.bibliography`, `bulk.polars.repeated-citations` |

A lane is one fair comparison contract. A group organizes related lanes. `capability` names the user behavior. `workflow` names the concrete path. `phase` and `operation_phase` name the measured boundary on successful rows. Both fields become `setup` on setup failure, so use `lane` to recover the intended operation.

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

Synthetic workloads contain 3, 48, and 192 entries for `tiny`, `medium`, and `large`. The tracked real workload contains 12 entries. Its provenance note and content hash preserve checkout-level reproducibility. A new real fixture needs reconstructable source identifiers, extraction date, hashes, item mapping, and license details.

`style.load` ignores workload contents and currently runs once per selected input. Treat those rows as repeated measurements of the same style operation, not as input-scaling evidence.

## Comparison Rules

- Schedule a package only when its public API owns the workflow.
- Keep setup placement equal inside one lane and record setup performed outside the measured rounds.
- Run a correctness check before accepting timing output.
- Preserve input identity, source license, record count, byte count, and hash in result rows.
- Use release-mode native builds for timing claims.
- Report failed and unsupported rows separately from successful timing comparisons.
- Back every performance claim with the command, inputs, runtime metadata, and result artifact that produced it.

## Runner Semantics

The runner executes in one process with deterministic participant and input order. It records one raw row per measured round with `perf_counter`. Correctness checks run during warmups and after every timed operation, outside the recorded `seconds` value.

A setup failure emits one zero-second failure row. A measured failure stops later rounds for that participant. Cleanup failure prints a diagnostic to standard error and leaves completed rows unchanged. The process exits nonzero when any row has `failed` status. Runs containing `ok` and `unsupported` rows exit zero.

`--build-mode release` records a caller-supplied label. Use `--build-mode auto` to read `refkit.build_mode`, and build both adapters with `maturin develop --release` before a timing claim.

## Measurement Limits

The runner preserves reproducible raw rounds. It does not aggregate results, calculate confidence intervals, randomize participant order, isolate each participant in a new process, control the garbage collector, or set CPU affinity.

Use the rows to inspect comparable workflows and prepare a dated analysis. A published performance claim should include environment, command, workload identity, raw result artifact, aggregation method, uncertainty, and known limitations.

The current comparison adapters cover `bibtexparser`, `citeproc-py`, and Pybtex where their public workflows overlap a RefKit lane. The [feature matrix](feature-matrix.md) records the broader inspected capability context.
