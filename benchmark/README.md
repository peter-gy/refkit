# refkit benchmarks

The benchmark runner compares `refkit`, `citeproc-py`, and `bibtexparser` through named citation and BibTeX workflows. Each timing row includes the case, package, phase, input size, round, elapsed seconds, package versions, Python version, OS, CPU, refkit commit, and native build mode.

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

`status` has three values:

- `ok`: the operation ran and its correctness check passed.
- `unsupported`: the package has no public API for that workflow.
- `failed`: setup, execution, or correctness failed.

Unsupported rows record API coverage and stay out of timing comparisons.

## Cases

| Case | Packages | Measured phase |
| --- | --- | --- |
| `bibtex_parse` | `refkit`, `citeproc-py`, `bibtexparser` | Parse a BibTeX file into each package's library model. |
| `raw_bibtex_roundtrip` | `refkit`, `bibtexparser` | Parse raw BibTeX, edit one title, and write BibTeX text. |
| `citation_render` | `refkit`, `citeproc-py` | Render one APA citation after setup. |
| `bibliography_render` | `refkit`, `citeproc-py` | Render an APA bibliography after setup. |
| `repeated_render` | `refkit`, `citeproc-py` | Render repeated APA citations after setup. |
| `one_off_cite` | `refkit`, `citeproc-py` | Read BibTeX, load APA, and render one citation. |
| `one_off_bibliography` | `refkit`, `citeproc-py` | Read BibTeX, load APA, and render a bibliography. |
| `missing_reference` | `refkit`, `citeproc-py` | Resolve one missing citation key. |
| `bulk_materialization` | `refkit`, `citeproc-py`, `bibtexparser` | Materialize parsed entries into Python-visible key and title rows. |
| `library_keys` | `refkit`, `citeproc-py`, `bibtexparser` | Enumerate all citation keys after setup. |
| `entry_lookup` | `refkit`, `citeproc-py`, `bibtexparser` | Look up or batch-project a fixed set of entry titles after setup. |
| `field_projection` | `refkit`, `citeproc-py`, `bibtexparser` | Project common scalar fields from entries after setup. Refkit uses `Library.project`, its public batch projection API. |

The runner separates setup from measured phases. Fixture materialization, input parsing for render cases, and style loading happen before warmups. Render cases create a fresh processor or document inside each measured operation so citation history does not carry across rounds. Parse cases measure parsing because parsing is the workflow under test. One-off cases include read, style load, and render because that is the workflow under test.

## Inputs

The tiny fixture is checked in at `benchmark/data/tiny.bib`. Tiny, medium, and large inputs are deterministic ordered slices of the same largest record set. The large input keeps the current largest record count.

Generated result files belong in `benchmark/results/`. Commit benchmark code and audited fixtures. Leave local result files ignored.
