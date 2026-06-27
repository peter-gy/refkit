# refkit-bench

`refkit-bench` measures bibliography capabilities across `refkit`, `polars-refkit`, and comparison packages. Each lane names one workflow, one measured phase, one source format, and the packages that own that workflow.

## Install And Build

`refkit-bench` is a uv workspace package. Its dependencies live in `packages/refkit-bench/pyproject.toml`.

Build the native extensions in release mode before collecting timing numbers:

```bash
uv sync --all-packages --group dev
(cd packages/refkit-core-py && uv run maturin develop --release)
(cd packages/polars-refkit && uv run maturin develop --release)
```

## Run Benchmarks

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
  --json packages/refkit-bench/results/smoke.json
```

Run the full suite and write JSON plus CSV output:

```bash
uv run --package refkit-bench python -m refkit_bench.runner \
  --group all \
  --input all \
  --rounds 5 \
  --warmups 2 \
  --json packages/refkit-bench/results/latest.json \
  --csv packages/refkit-bench/results/latest.csv
```

Generated result files belong in `packages/refkit-bench/results/`. Commit benchmark code and audited fixtures. Leave local result files ignored.

## Lane Model

Use lane identity before comparing timings. Rows with different `lane`, `source_format`, `setup_included`, or `execution_mode` answer different questions.

| Field | Meaning |
| --- | --- |
| `lane`, `group`, `capability`, `workflow` | The public capability and workflow under measurement. |
| `package`, `package_version`, `adapter_version` | The package under test and its adapter version. |
| `phase`, `operation_phase` | The lane-owned measured phase. |
| `input`, `input_size`, `workload_family`, `source_name`, `source_path`, `source_license`, `record_count`, `input_bytes`, `input_sha256` | Workload identity, provenance, size, and fingerprint. |
| `failed_block_count`, `diagnostic_count` | Parser recovery counts reported by lanes that expose them. |
| `source_format` | One of `bibtex`, `raw_bibtex`, `dirty_bibtex`, `csl_json`, `none`, or `unknown`. |
| `citation_count` | Citation requests issued by the operation. |
| `execution_mode` | `eager` or `lazy` for Polars rows. Blank for non-Polars adapters. |
| `setup_included`, `setup_seconds` | Whether setup work is inside measured time, plus setup time when recorded outside measured rounds. |
| `rounds`, `warmups`, `round`, `seconds`, `status`, `operation_count` | Run settings and measured output. |
| `python`, `os`, `cpu`, `refkit_version`, `refkit_commit`, `build_mode` | Runtime and source context. |

`status` has three values:

| Status | Meaning |
| --- | --- |
| `ok` | The operation ran and its correctness check passed. |
| `failed` | Setup, execution, or correctness failed. |
| `unsupported` | The adapter declared that the lane or workload is outside its public contract before timing. |

## Lane Families

| Group | Lanes | Participants |
| --- | --- | --- |
| `input.normalized` | `input.bibtex-text`, `input.bibtex`, `input.dirty-bibtex`, `input.diagnostics` | `refkit`, `bibtexparser-2.x`, and `pybtex` for clean BibTeX text. `polars-refkit` for clean BibTeX rows. `refkit` and `bibtexparser-2.x` for recovery and diagnostics. |
| `raw-bibtex` | `raw-bibtex.parse`, `raw-bibtex.blocks`, `raw-bibtex.duplicates`, `raw-bibtex.write`, `raw-bibtex.roundtrip` | `refkit`, `bibtexparser-2.x` |
| `style` | `style.load`, `style.processor-setup` | `refkit`, `citeproc-py` |
| `render.prepared` | prepared citation, bibliography, cited bibliography, repeated citations | `refkit`, `citeproc-py` |
| `render.one-off` | `render.one-off-cite`, `render.one-off-bibliography` | `refkit`, `citeproc-py` |
| `render.output` | `render.output-text`, `render.output-html`, `render.output-tree` | `refkit` |
| `inspect.entries` | `inspect.materialize`, `inspect.keys`, `inspect.lookup`, `inspect.fields` | `refkit`, `bibtexparser-2.x`, `pybtex` |
| `bulk.polars` | materialize, keys, lookup, fields, citation, bibliography, repeated citations | `polars-refkit` eager and lazy variants |
| `errors` | `errors.missing-reference` | `refkit`, `citeproc-py` |

Prepared render lanes compare citation processors after source and style setup. Path-based one-off render lanes include file read, parse, style load, processor setup, and render. Polars expression rows parse BibTeX and render inside the Polars operation, so they measure dataframe workflows rather than prepared processor calls.

## Inputs

The benchmark uses generated scale fixtures and one real corpus fixture.

| Input | Source |
| --- | --- |
| `tiny`, `medium`, `large` | Deterministic ordered slices of the generated record set. |
| `real` | `src/refkit_bench/data/real-bibliography/references.bib`, a compact real-world BibTeX subset used by parser stress tests. |

Each workload has five source forms:

| Source form | Meaning |
| --- | --- |
| `bibtex` | Clean BibTeX for the selected workload. |
| `raw_bibtex` | BibTeX with comments, a string definition, and a preamble for generated workloads. The real bibliography workload uses the same real BibTeX as its raw input. |
| `dirty_bibtex` | Generated BibTeX with malformed input, a duplicate key, invalid month, and unresolved abbreviation. The real bibliography workload uses the same real BibTeX because it is a clean corpus slice. |
| `duplicate_bibtex` | Generated BibTeX with a duplicate entry key and a duplicate field in separate entries. |
| `csl_json` | CSL JSON used by citeproc-py prepared render lanes. |

`refkit-bench` installs `bibtexparser==2.0.0b9`, `citeproc-py>=0.9.3`, and `pybtex>=0.26,<0.27` as comparison points.

## License

`refkit-bench` is licensed under the Apache License, Version 2.0, available in [LICENSE](LICENSE). See [NOTICE](NOTICE) for upstream citation and bibliography component acknowledgements.
