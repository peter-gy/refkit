# refkit benchmarks

The benchmark runner compares `refkit`, `polars-refkit`, `citeproc-py`, and `bibtexparser` 2 through named citation and BibTeX workflows. The benchmark group installs `bibtexparser==2.0.0b9` as the current prerelease comparison point.

The harness is a local smoke benchmark. It uses `time.perf_counter()` around the measured operation. Use the saved JSON and CSV to compare local changes. Do not treat one run as a statistically publishable result.

Install benchmark dependencies and build the native extension in release mode:

```bash
uv sync --all-packages --group benchmark
uv run maturin develop --release --manifest-path packages/refkit/Cargo.toml
uv run maturin develop --release --manifest-path packages/polars-refkit/Cargo.toml
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
- `package`, `package_version`, and `adapter_version`: the adapter under test and comparison version.
- `input`, `input_size`, `workload_family`, `record_count`, `input_bytes`, and `input_sha256`: the workload identity and input fingerprint.
- `source_format`: `bibtex`, `raw_bibtex`, `dirty_bibtex`, `csl_json`, `none`, `unsupported`, or `unknown`.
- `citation_count`: number of citation requests issued by the operation. All-entry bibliography APIs that do not issue citation requests record `0`. `operation_count` records rendered entries.
- `execution_mode`: `eager` or `lazy` for Polars expression rows. It is blank for non-Polars adapters and unsupported rows.
- `setup_included`: whether parsing, style loading, or similar setup work is part of measured `seconds`.
- `setup_seconds`: adapter preparation time recorded by the runner outside measured rounds.
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
| `bibtex_parse` | `refkit`, `polars-refkit`, `citeproc-py`, `bibtexparser-2.x` | Parse BibTeX into each package's library model or dataframe expression result. The Polars case includes file read, dataframe creation, and plugin execution in the measured operation. |
| `bibtex_recovery_parse` | `refkit`, `bibtexparser-2.x` | Parse dirty BibTeX and require valid entries to survive. v2 returns surviving entries plus failed blocks for the fixture. citeproc-py aborts on the dirty fixture and is reported as unsupported for this case. |
| `raw_bibtex_parse` | `refkit`, `bibtexparser-2.x` | Parse raw BibTeX without writing output. |
| `raw_bibtex_write` | `refkit`, `bibtexparser-2.x` | Write an already parsed raw BibTeX document after one field edit. |
| `raw_bibtex_roundtrip` | `refkit`, `bibtexparser-2.x` | Parse raw BibTeX, edit one title, and write BibTeX text. |
| `style_load` | `refkit`, `citeproc-py` | Resolve the APA style after runner warmup. Public process caches count as part of this steady-state row. |
| `processor_setup` | `refkit`, `citeproc-py` | Create a document or processor from already prepared inputs. |
| `citation_render` | `refkit`, `polars-refkit`, `citeproc-py` | Render one APA citation. refkit and citeproc-py use prepared citation data with `setup_included=false`. The Polars row is a BibTeX expression workflow with `setup_included=true`, so it parses the BibTeX row and renders inside the measured operation. |
| `bibliography_render` | `refkit`, `polars-refkit`, `citeproc-py` | Render one APA bibliography. refkit and citeproc-py use prepared citation data with `setup_included=false`. The Polars row is a BibTeX expression workflow with `setup_included=true` and uses `bibliography_text` over one full BibTeX row. |
| `bibliography_seen_render` | `refkit`, `citeproc-py` | Cite every entry during the operation, then render the cited bibliography. |
| `repeated_render` | `refkit`, `polars-refkit`, `citeproc-py` | Render repeated APA citations. refkit and citeproc-py use prepared citation data with `setup_included=false`. The Polars row is a BibTeX expression workflow with `setup_included=true` and uses one BibTeX row plus one `List[String]` key row with `cite_sequence`. |
| `rendered_text_access` | `refkit` | Access `.text` from an already rendered citation. |
| `rendered_html_access` | `refkit` | Access `.html` from an already rendered citation. |
| `rendered_tree_access` | `refkit` | Materialize `.tree` from an already rendered citation. |
| `one_off_cite` | `refkit`, `citeproc-py` | Call the one-off citation helper after runner warmup. The measured operation reads BibTeX, resolves APA through the package's public loader, and renders one citation. |
| `one_off_bibliography` | `refkit`, `citeproc-py` | Call the one-off bibliography helper after runner warmup. The measured operation reads BibTeX, resolves APA through the package's public loader, and renders a bibliography. |
| `missing_reference` | `refkit`, `citeproc-py` | Resolve one missing citation key. |
| `bulk_materialization` | `refkit`, `polars-refkit`, `citeproc-py`, `bibtexparser-2.x` | Materialize entries into key and title rows. The Polars case measures the `entries` expression from one full BibTeX row. |
| `library_keys` | `refkit`, `polars-refkit`, `citeproc-py`, `bibtexparser-2.x` | Enumerate all citation keys. The Polars case measures the `keys` expression from one full BibTeX row. |
| `entry_lookup` | `refkit`, `polars-refkit`, `citeproc-py`, `bibtexparser-2.x` | Look up or batch-project a fixed set of entry titles after setup. The Polars case measures native entry projection from one full BibTeX row, then filters the fixed lookup key set inside Polars. |
| `field_projection` | `refkit`, `polars-refkit`, `citeproc-py`, `bibtexparser-2.x` | Project common scalar fields from entries. Refkit uses `Library.project`. `polars-refkit` measures native `entries` projection from one full BibTeX row. |
| `lazy_bibtex_parse` | `polars-refkit` | Parse BibTeX through a lazy Polars expression and collect the result. |
| `lazy_citation_render` | `polars-refkit` | Render one APA citation through a lazy Polars expression and collect the result. |
| `lazy_bibliography_render` | `polars-refkit` | Render an APA bibliography through a lazy Polars expression and collect the result. |
| `lazy_repeated_render` | `polars-refkit` | Render an ordered citation batch through a lazy Polars expression and collect the result. |
| `lazy_bulk_materialization` | `polars-refkit` | Materialize entry rows through a lazy Polars expression and collect the result. |
| `lazy_library_keys` | `polars-refkit` | Enumerate citation keys through a lazy Polars expression and collect the result. |
| `lazy_entry_lookup` | `polars-refkit` | Project and filter entries through a lazy Polars expression and collect the result. |
| `lazy_field_projection` | `polars-refkit` | Project common scalar fields through a lazy Polars expression and collect the result. |

The runner separates setup from measured phases. Fixture materialization, loaded-library render setup, and style resolution happen before warmups when `setup_included` is false. Warmups are part of the timing contract. Rows such as `style_load`, `one_off_cite`, and `one_off_bibliography` are steady-state rows inside one Python process. If a package caches parsed styles through its public API, measured rounds use that cache after warmup. Do not present those rows as first-call cold-start timings.

Render cases create a fresh processor or document inside each measured operation so citation history does not carry across rounds. `repeated_render` uses every key in the selected input size, so medium and large rows exercise longer citation histories. `polars-refkit` uses `cite_sequence` for that case so the benchmark measures its list-of-keys citation path.

Reports must segment or label rows by `setup_included`, `source_format`, and `execution_mode` before declaring cross-package winners. In particular, do not present Polars BibTeX expression render rows as the same prepared-render contract as refkit or citeproc-py. Compare those Polars rows as plugin expression workflows, or use warmed one-off rows when the question is steady-state BibTeX-to-output helper time.

Parse and raw roundtrip cases measure parsing because parsing is the workflow under test. One-off cases include read, style load, and render because that is the workflow under test.

`citeproc-py` render cases use CSL JSON for loaded-library render setup. Its one-off render cases read BibTeX inside the measured operation. `polars-refkit` uses full BibTeX rows for parse, key, materialization, lookup, field projection, citation render, and bibliography render cases. Eager Polars rows call `DataFrame.select`. Lazy Polars rows call `LazyFrame.select(...).collect()`. Polars expression rows mark `setup_included=true` because the native plugin parses BibTeX, and for render expressions loads the citation style, during the measured operation. Projection and key cases build the input dataframe during setup, then measure the Polars plugin expression and checked-row collection during each operation. `bibtexparser-2.x` uses the installed `parse_file`, `parse_string`, and `write_string` APIs, and setup fails unless the installed package is exactly `bibtexparser==2.0.0b9`. Its dirty BibTeX case checks recovered entry count plus the expected `ParsingFailedBlock:missing` and `DuplicateBlockKeyBlock:<key>` signatures for the fixture. `polars-refkit` does not run raw malformed-document recovery. The `source_format`, `execution_mode`, and `setup_included` fields record these differences per row.

Use phase cases to localize regressions before comparing total workflow time:

- `bibtex_parse` measures clean file parsing.
- `bibtex_recovery_parse` measures dirty BibTeX recovery. Packages that abort without recovered entries are reported as unsupported for this case.
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
