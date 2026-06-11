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
- `source_format`: `bibtex`, `raw_bibtex`, `dirty_bibtex`, `csl_json`, `none`, `unsupported`, or `unknown`.
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
| `bibtex_recovery_parse` | `refkit`, `citeproc-py`, `bibtexparser-1.x` | Parse dirty BibTeX and record whether valid entries survive. |
| `raw_bibtex_parse` | `refkit`, `bibtexparser-1.x` | Parse raw BibTeX without writing output. |
| `raw_bibtex_write` | `refkit`, `bibtexparser-1.x` | Write an already parsed raw BibTeX document after one field edit. |
| `raw_bibtex_roundtrip` | `refkit`, `bibtexparser-1.x` | Parse raw BibTeX, edit one title, and write BibTeX text. |
| `style_load` | `refkit`, `citeproc-py` | Load the APA style. |
| `processor_setup` | `refkit`, `citeproc-py` | Create a document or processor from already prepared inputs. |
| `citation_render` | `refkit`, `citeproc-py` | Render one APA citation after setup. |
| `bibliography_render` | `refkit`, `citeproc-py` | Render an APA bibliography after setup. |
| `bibliography_seen_render` | `refkit`, `citeproc-py` | Cite every entry during the operation, then render the cited bibliography. |
| `repeated_render` | `refkit`, `citeproc-py` | Render repeated APA citations after setup. |
| `rendered_text_access` | `refkit` | Access `.text` from an already rendered citation. |
| `rendered_html_access` | `refkit` | Access `.html` from an already rendered citation. |
| `rendered_tree_access` | `refkit` | Materialize `.tree` from an already rendered citation. |
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

Use phase cases to localize regressions before comparing total workflow time:

- `bibtex_parse` measures clean file parsing.
- `bibtex_recovery_parse` measures dirty BibTeX recovery and records failures for packages that cannot recover the fixture.
- `raw_bibtex_parse`, `raw_bibtex_write`, and `raw_bibtex_roundtrip` separate raw scan, writeback, and full edit workflows.
- `style_load` and `processor_setup` separate style setup from citation rendering.
- `citation_render`, `repeated_render`, `bibliography_render`, and `bibliography_seen_render` separate first citation, citation history, all-entry bibliography, and cited-entry bibliography paths.
- `rendered_text_access`, `rendered_html_access`, and `rendered_tree_access` isolate refkit output materialization after rendering.

## Inputs

The tiny fixture is checked in at `benchmark/data/tiny.bib`. Tiny, medium, and large inputs are deterministic ordered slices of the same largest record set. The large input keeps the current largest record count. Current generated workloads use the `synthetic_scale` family.

Each workload has four source forms:

- `bibtex`: clean generated BibTeX.
- `raw_bibtex`: generated BibTeX with comments, a string definition, and a preamble.
- `dirty_bibtex`: generated BibTeX with a malformed block, duplicate key, invalid month, and unresolved abbreviation.
- `csl_json`: CSL JSON used by citeproc-py loaded-source render cases.

Generated result files belong in `benchmark/results/`. Commit benchmark code and audited fixtures. Leave local result files ignored.
