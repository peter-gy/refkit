# refkit benchmarks

The benchmark runner measures capability lanes. A lane names the capability, workflow, source format, setup contract, measured phase, and packages that own that workflow. Execution mode is row metadata, so eager and lazy Polars variants share the same lane.

`refkit-bench` is a uv workspace package. Its dependencies live in `benchmark/pyproject.toml`.

Install workspace dependencies and build the native extensions in release mode:

```bash
uv sync --all-packages --group dev
uv run maturin develop --release --manifest-path packages/refkit/Cargo.toml
uv run maturin develop --release --manifest-path packages/polars-refkit/Cargo.toml
```

List lanes:

```bash
uv run --package refkit-bench python -m refkit_bench.runner --list
```

Run one lane and write JSON output:

```bash
uv run --package refkit-bench python -m refkit_bench.runner \
  --lane input.bibtex \
  --rounds 3 \
  --warmups 1 \
  --json benchmark/results/smoke.json
```

Run the full suite and write JSON plus CSV output:

```bash
uv run --package refkit-bench python -m refkit_bench.runner \
  --group all \
  --input all \
  --rounds 5 \
  --warmups 2 \
  --json benchmark/results/latest.json \
  --csv benchmark/results/latest.csv
```

## Result Rows

Saved rows are self-describing. A row includes:

- `lane`, `group`, `capability`, and `workflow`: the capability lane and measured public workflow.
- `package`, `package_version`, and `adapter_version`: the package under test and comparison version.
- `phase` and `operation_phase`: the lane-owned measured phase.
- `input`, `input_size`, `workload_family`, `source_name`, `source_path`, `source_license`, `record_count`, `input_bytes`, and `input_sha256`: the workload identity, source provenance, and input fingerprint.
- `failed_block_count` and `diagnostic_count`: parser recovery counts reported by lanes that expose them. They are `0` when the lane does not report that data.
- `source_format`: `bibtex`, `raw_bibtex`, `dirty_bibtex`, `csl_json`, `none`, or `unknown`.
- `citation_count`: citation requests issued by the operation. All-entry bibliography lanes record `0` unless the workflow cites entries first.
- `execution_mode`: `eager` or `lazy` for Polars expression rows. It is blank for non-Polars adapters.
- `setup_included`: whether parsing, style loading, dataframe expression work, or similar setup work is part of measured `seconds`.
- `setup_seconds`: adapter preparation time recorded by the runner outside measured rounds.
- `rounds`, `warmups`, `round`, `seconds`, `status`, and `operation_count`: run settings and measured output.
- `python`, `os`, `cpu`, `refkit_version`, `refkit_commit`, and `build_mode`: runtime and source context.

`status` has two values:

- `ok`: the operation ran and its correctness check passed.
- `failed`: setup, execution, or correctness failed.

## Lane Families

| Group              | Lanes                                                                                                                                                                           | Participants                                                                                      |
| ------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `input.normalized` | `input.bibtex-text`, `input.bibtex`, `input.dirty-bibtex`                                                                                                                       | `refkit`, `bibtexparser-2.x`, and `pybtex` for clean BibTeX text, `polars-refkit` for clean BibTeX rows, `refkit` and `bibtexparser-2.x` for recovery |
| `raw-bibtex`       | `raw-bibtex.parse`, `raw-bibtex.write`, `raw-bibtex.roundtrip`                                                                                                                  | `refkit`, `bibtexparser-2.x`                                                                      |
| `style`            | `style.load`, `style.processor-setup`                                                                                                                                           | `refkit`, `citeproc-py`                                                                           |
| `render.prepared`  | prepared citation, bibliography, cited bibliography, repeated citations                                                                                                         | `refkit`, `citeproc-py`                                                                           |
| `render.one-off`   | `render.one-off-cite`, `render.one-off-bibliography`                                                                                                                            | `refkit`, `citeproc-py`                                                                           |
| `render.output`    | `render.output-text`, `render.output-html`, `render.output-tree`                                                                                                                | `refkit`                                                                                          |
| `inspect.entries`  | `inspect.materialize`, `inspect.keys`, `inspect.lookup`, `inspect.fields`                                                                                                       | `refkit`, `bibtexparser-2.x`, `pybtex`                                                            |
| `bulk.polars`      | `bulk.polars.materialize`, `bulk.polars.keys`, `bulk.polars.lookup`, `bulk.polars.fields`, `bulk.polars.citation`, `bulk.polars.bibliography`, `bulk.polars.repeated-citations` | `polars-refkit` eager and lazy variants                                                           |
| `errors`           | `errors.missing-reference`                                                                                                                                                      | `refkit`, `citeproc-py`                                                                           |

The runner schedules only the participants declared by a lane. `citeproc-py` appears in rendering, style setup, processor setup, one-off render, and missing-reference lanes. `bibtexparser==2.0.0b9` appears in BibTeX parse, raw document, writeback, roundtrip, key lookup, field projection, and materialization lanes. `pybtex` appears in clean BibTeX parse and inspection lanes.

## Fair Comparisons

Use lane identity before declaring a winner. Rows with different `lane`, `source_format`, `setup_included`, or `execution_mode` answer different questions.

Prepared render lanes compare citation processors after source and style setup. Path-based one-off render lanes include file read, parse, style load, processor setup, and render. Polars expression rows parse BibTeX and render inside the Polars operation, so they measure dataframe workflows rather than prepared processor calls. Compare eager and lazy Polars rows through `execution_mode`, not by changing the lane.

`refkit-bench` installs `bibtexparser==2.0.0b9` and `pybtex>=0.26,<0.27` as Python BibTeX comparison points.

## Inputs

The tiny fixture is packaged at `benchmark/src/refkit_bench/data/tiny.bib`. Tiny, medium, and large inputs are deterministic ordered slices of the largest generated record set. Current generated workloads use the `synthetic_scale` family and report `Apache-2.0` as the source license.

Each workload has four source forms:

- `bibtex`: clean generated BibTeX.
- `raw_bibtex`: generated BibTeX with comments, a string definition, and a preamble.
- `dirty_bibtex`: generated BibTeX with a malformed block, duplicate key, invalid month, and unresolved abbreviation.
- `csl_json`: CSL JSON used by citeproc-py prepared render lanes.

Generated result files belong in `benchmark/results/`. Commit benchmark code and audited fixtures. Leave local result files ignored.
