---
description: Inspect a dated RefKit parsing and citation-rendering measurement with its workload, command, environment, and limits.
---

# Performance

RefKit moves bibliography parsing and rendering into an in-process Rust core. A focused run on September 6, 2026 measured two small Python workflows from the public benchmark runner.

## Results

| Lane | Package | Median |
| --- | --- | ---: |
| Parse three clean BibTeX entries from an in-memory string | RefKit 0.0.4rc5 | 41.4 µs |
| Parse the same string | [bibtexparser](https://github.com/sciunto-org/python-bibtexparser) 2.0.0b9 | 90.3 µs |
| Parse the same string | [Pybtex](https://pybtex.org/) 0.26.1 | 180.5 µs |
| Render one prepared [APA](https://apastyle.apa.org/) citation | RefKit 0.0.4rc5 | 321.8 µs |
| Render the same prepared citation | [citeproc-py](https://github.com/citeproc-py/citeproc-py) 0.10.1 | 1,350.9 µs |

Each median covers 12 measured rounds after three warmups. The runner checked the expected result after every timed operation.

[Inspect the 60 raw result rows →](/benchmarks/refkit-0.0.4rc5-macos-arm64-2026-09-06.json)

## Reproduce the run

Install [Git](https://git-scm.com/), [uv](https://docs.astral.sh/uv/), and the [Rust toolchain](https://www.rust-lang.org/tools/install). The recorded run used Python 3.12. Start from the repository revision that owns the benchmark inputs and adapters:

```bash
git clone https://github.com/peter-gy/refkit.git
cd refkit
git checkout 7110d53705db9da2194c4e8615789772e04f0df3
uv sync --locked --all-packages --group dev
```

Build the native Python adapter in release mode from the checkout root:

```bash
(cd packages/refkit && uv run maturin develop --release)
```

Run the two lanes:

```bash
uv run --locked --package refkit-bench python -m refkit_bench.runner \
  --lane input.bibtex-text \
  --lane render.prepared-citation \
  --input tiny \
  --rounds 12 \
  --warmups 3 \
  --build-mode auto \
  --json packages/refkit-bench/results/performance.json
```

The published result file normalizes the temporary source path to `synthetic/tiny.bib`. Input bytes, source identity, SHA-256 hash, package versions, build mode, setup time, per-round time, and runtime metadata remain unchanged.

The run used RefKit revision `7110d53`, Python 3.12.0, macOS 26.6.2, and an Apple M3 Max with 36 GB of memory. The `tiny` workload contains three entries. `input.bibtex-text` includes in-memory parse and normalization. `render.prepared-citation` begins after bibliography and style setup.

## Interpret the numbers

These measurements describe two small workflows on one machine. They do not establish latency for large files, recovery, raw edits, full bibliographies, rendered trees, Polars plans, source builds, or Pyodide.

The runner uses one process and deterministic participant order. It records raw rounds without process isolation, CPU affinity, or confidence intervals. Re-run the matching lane on the workload and host that control your decision.

Read [How RefKit Works](/concepts/how-refkit-works) for the architecture behind these paths.
