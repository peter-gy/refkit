# refkit benchmarks

The benchmark runner compares `refkit`, `citeproc-py`, and the `bibtexparser` 1.x API through named citation and BibTeX workflows. Each result row records the operation, package version, workload shape, source format, setup policy, timing, Python runtime, OS, CPU, refkit commit, and native build mode.

The harness is a local smoke benchmark. It uses `time.perf_counter()` around the measured operation. Use the saved JSON and CSV to compare local changes. Do not treat one run as a statistically publishable result.

Install benchmark dependencies and build the native extension in release mode:

```bash
uv sync --group benchmark
uv run maturin develop --release
```

List the available cases:

```bash
uv run python -m benchmark.runner --list
```

Run one benchmark and write JSON output:

```bash
uv run python -m benchmark.runner \
  --case bibtex_parse \
  --rounds 3 \
  --warmups 1 \
  --json benchmark/results/smoke.json
```

Run the full suite and write JSON plus CSV output:

```bash
uv run python -m benchmark.runner \
  --group all \
  --rounds 5 \
  --warmups 2 \
  --json benchmark/results/latest.json \
  --csv benchmark/results/latest.csv
```

## Result Rows

Saved rows are self-describing. A row includes:

- `case`, `group`, `phase`, and `operation_phase`: the benchmarked workflow and operation phase.
- `package`, `package_version`, and `adapter_version`: the adapter under test and installed distribution version.
- `input`, `input_size`, `workload_family`, `record_count`, `input_bytes`, and `input_sha256`: the workload identity and input fingerprint.
- `source_format`: `bibtex`, `raw_bibtex`, `csl_json`, `unsupported`, or `unknown`.
- `citation_count`: number of citation requests issued by the operation. All-entry bibliography APIs that do not issue citation requests record `0`. `operation_count` records rendered entries.
- `setup_included`: whether parsing, style loading, or similar setup work is part of measured `seconds`.
- `setup_seconds`: adapter preparation time recorded outside measured rounds.
- `rounds`, `warmups`, `round`, `seconds`, `status`, and `operation_count`: run settings and measured output.
- `python`, `os`, `cpu`, `refkit_version`, `refkit_commit`, and `build_mode`: runtime and source context.

`status` has three values:

- `ok`: the operation ran and its correctness check passed.
- `unsupported`: the package has no public API for that workflow.
- `failed`: setup, execution, or correctness failed.

Unsupported rows record API coverage and stay out of timing comparisons.

## Cases

| Case | Packages | Measured phase |
| --- | --- | --- |
| `bibtex_parse` | `refkit`, `citeproc-py`, `bibtexparser-1.x` | Parse a BibTeX file into each package's library model. |
| `raw_bibtex_roundtrip` | `refkit`, `bibtexparser-1.x` | Parse raw BibTeX, edit one title, and write BibTeX text. |
| `citation_render` | `refkit`, `citeproc-py` | Render one APA citation after setup. |
| `bibliography_render` | `refkit`, `citeproc-py` | Render an APA bibliography after setup. |
| `repeated_render` | `refkit`, `citeproc-py` | Render repeated APA citations after setup. |
| `one_off_cite` | `refkit`, `citeproc-py` | Read BibTeX, load APA, and render one citation. |
| `one_off_bibliography` | `refkit`, `citeproc-py` | Read BibTeX, load APA, and render a bibliography. |
| `missing_reference` | `refkit`, `citeproc-py` | Resolve one missing citation key. |
| `bulk_materialization` | `refkit`, `citeproc-py`, `bibtexparser-1.x` | Materialize parsed entries into Python-visible key and title rows. |
| `library_keys` | `refkit`, `citeproc-py`, `bibtexparser-1.x` | Enumerate all citation keys after setup. |
| `entry_lookup` | `refkit`, `citeproc-py`, `bibtexparser-1.x` | Look up or batch-project a fixed set of entry titles after setup. |
| `field_projection` | `refkit`, `citeproc-py`, `bibtexparser-1.x` | Project common scalar fields from entries after setup. Refkit uses `Library.project`, its public batch projection API. |

The runner separates setup from measured phases. Fixture materialization, loaded-library render setup, and style loading happen before warmups when `setup_included` is false.

Render cases create a fresh processor or document inside each measured operation so citation history does not carry across rounds. `repeated_render` uses every key in the selected input size, so medium and large rows exercise longer citation histories.

Parse and raw roundtrip cases measure parsing because parsing is the workflow under test. One-off cases include read, style load, and render because that is the workflow under test.

`citeproc-py` render cases use CSL JSON for loaded-library render setup. Its one-off render cases read BibTeX inside the measured operation. The `source_format` field records that difference per row.

## Inputs

The tiny fixture is checked in at `benchmark/data/tiny.bib`. Tiny, medium, and large inputs are deterministic ordered slices of the same largest record set. The large input keeps the current largest record count. Current generated workloads use the `synthetic_scale` family.

Generated result files belong in `benchmark/results/`. Commit benchmark code and audited fixtures. Leave local result files ignored.
