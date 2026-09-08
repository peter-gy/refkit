---
description: Measure RefKit capabilities with fixed inputs, correctness checks, isolated workers, and reproducible baseline comparisons.
---

# Performance

RefKit's benchmark measures public bibliography operations with fixed inputs and checked outputs. Choose the capability that controls your workload: parsing, entry lookup, BibTeX edits, citation rendering, formatting, or Polars batch execution.

## Measure a capability

The runner lives in `packages/refkit-bench` in the [source repository](https://github.com/peter-gy/refkit). Follow the [benchmark setup and methodology](https://github.com/peter-gy/refkit/blob/main/development_docs/benchmarks.md) to install the participants and build release-mode adapters.

From that checkout:

```bash
uv run --no-sync refkit-bench check --lane parse.bibtex --dataset real
uv run --no-sync refkit-bench run --lane parse.bibtex --dataset real \
  --output packages/refkit-bench/results/baseline
uv run --no-sync refkit-bench report packages/refkit-bench/results/baseline
```

This case parses the same 12-record bibliography through RefKit, [bibtexparser](https://github.com/sciunto-org/python-bibtexparser), and [Pybtex](https://pybtex.org/). Validation checks complete metadata. Timings cover in-memory model creation, with validation outside the timer.

## Read the evidence

[pyperf](https://pyperf.readthedocs.io/), a Python benchmarking tool, calibrates batches and collects repeated values in separate workers. A result directory contains:

- `manifest.json`: input and artifact hashes, source provenance, setup boundaries, validation outcomes, runtime settings, and host identity.
- `timings.json`: measured values, calibration, warmups, and worker metadata in pyperf's format.

The summary reports the median of worker means for each case. Compare matched runs with:

```bash
uv run --no-sync refkit-bench compare \
  packages/refkit-bench/results/baseline \
  packages/refkit-bench/results/candidate
```

The ratio is candidate time divided by baseline time. Values below 1 mean lower elapsed time. With enough independent workers, the report includes exploratory per-case bootstrap intervals. Repeat small effects in alternating baseline/candidate sessions on the same idle machine before attributing a gain to an implementation change.

## Match the workflow

Parsing, rendering, formatting, and dataframe execution have different units and setup costs. Compare within a lane and dataset. The shared rendering fixture checks exact basic author-date output. Polars batch citation cases use the plugin's bundled APA style. The formatter corpus checks exact expectations from a pinned [bibtex-tidy](https://github.com/FlamingTempura/bibtex-tidy) revision and records differences from the published package.

The methodology guide lists every lane, its timed boundary, corpus coverage, known conformance differences, and commands for scaling experiments. A failed correctness check prevents that selected run from producing a timing comparison.
