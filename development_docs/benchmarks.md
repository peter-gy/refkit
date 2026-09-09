# Benchmarks

`refkit-bench` measures complete public operations: parsing a bibliography, looking up entries, editing BibTeX, rendering citations, formatting source, and executing Polars queries. A case identifies one **lane**, **dataset**, and **participant**, such as `parse.bibtex/real/refkit`.

Start with the operation you want to improve. Keep its inputs and output contract fixed while changing RefKit.

## First run

Install [uv](https://docs.astral.sh/uv/), the Python workspace manager, [Rust](https://www.rust-lang.org/tools/install), and [Node.js](https://nodejs.org/) 26 for the JavaScript formatter participant. From the checkout root:

```bash
make sync
npm ci --ignore-scripts --prefix packages/refkit-bench/node
make refkit-develop-release polars-refkit-develop-release
uv run --no-sync refkit-bench list --lane parse.bibtex --dataset real
uv run --no-sync refkit-bench check --lane parse.bibtex --dataset real
uv run --no-sync refkit-bench run --lane parse.bibtex --dataset real \
  --output packages/refkit-bench/results/baseline
uv run --no-sync refkit-bench report packages/refkit-bench/results/baseline
```

The check command reports each participant's result. The run command creates a fresh directory containing `manifest.json` and `timings.json`, then prints elapsed time per complete operation. Existing result directories are preserved.

Use `--no-sync` after building release adapters. Dependency synchronization can rebuild editable adapters with debug settings. Measurement runs require the native modules to report `release`. `--smoke` accepts debug builds and runs one short worker to verify the harness. Smoke results cannot be used by `compare`.

## Choose a capability

`--lane`, `--dataset`, and `--package` can be repeated. `list --lane all --dataset all` displays every applicable case. Explicit dataset selection restricts the case set, so `--dataset tiny` selects synthetic workloads and `--lane format.spec` selects named specification fixtures.

| Lane | Participants | Measured operation | Setup outside timing |
| --- | --- | --- | --- |
| `parse.bibtex` | refkit, bibtexparser, pybtex | Parse identical in-memory BibTeX into the package's queryable model. | Load source and validation resources. |
| `inspect.keys` | refkit, bibtexparser, pybtex | List all keys. | Parse the library. |
| `inspect.lookup` | refkit, bibtexparser, pybtex | Fetch first, middle, and last entries as key/title rows. | Parse the library. |
| `inspect.project` | refkit, bibtexparser, pybtex | Materialize key, title, DOI, and volume for every entry. | Parse the library. |
| `raw.edit` | refkit, bibtexparser | Parse, change one title, and serialize to memory. | Load source and expected fields/blocks. |
| `render.citation` | refkit, citeproc-py | Create a processor and request, render one citation, materialize text. | Parse equivalent source data and the shared style. |
| `render.bibliography` | refkit, citeproc-py | Create a processor and render the complete ordered bibliography. | Parse equivalent source data and the shared style. |
| `render.document` | refkit, citeproc-py | Create a processor and requests, render all citations and bibliography. | Parse equivalent source data and the shared style. |
| `format.layout` | refkit, bibtex-tidy | Parse, transform, and format with the explicit shared layout profile. | Load source and construct options. |
| `format.keys` | refkit, bibtex-tidy | Format with year-based keys and alphabetic collision suffixes. | Load source and the explicit profile. |
| `format.spec` | refkit, bibtex-tidy | Apply one upstream specification's options to its input. | Load the exact expected text and options. |
| `batch.parse` | polars-eager, polars-lazy | Build and execute an expression, then materialize projected entry rows. | Construct a dataframe of independent bibliographies. |
| `batch.cite` | polars-eager, polars-lazy | Build and execute an APA citation expression, then materialize rows. | Construct a dataframe of independent bibliographies. |

Inspection uses package-owned lookup APIs. Any lookup map materialization required by that API stays inside the operation. Every edit starts from a fresh document. Polars uses one entry per bibliography row and records its actual row count. `--polars-threads` fixes the thread pool before import in both validation and measurement workers, with a default of one.

## Inputs and correctness

| Dataset | Contents | Purpose |
| --- | --- | --- |
| `tiny`, `medium`, `large` | 3, 48, and 192 generated records | Fixed-shape scaling and quick local checks. |
| `real` | 12 curated scholarly records with complete author lists | Shared realistic metadata for parsing and rendering. |
| `1k`, `5k`, `10k` | 1,000, 5,000, and 10,000 generated records | Explicit larger scaling experiments. |
| Named `format.spec` cases | 16 upstream inputs, option sets, and exact outputs | Numeric preservation, concatenation, paragraphs, braces, escaping, field cleanup, sorting, duplicates, and generated keys. |

The synthetic records hold field shape constant as entry count grows. They isolate size scaling for that shape. The real metadata's sources and license are in [its provenance note](../packages/refkit-bench/src/refkit_bench/data/real-bibliography/README.md).

Parsing checks require key, type, title, ordered full author names, year, container, volume, pages, and DOI. RefKit exposes authors and pages through rendering, so its checker renders the **actual parsed library** through the authored validation style outside timing. The other parsers expose those fields directly. Raw editing checks all entry fields and the preserved comment, string definition, and preamble.

Rendering participants consume the same [Citation Style Language](https://citationstyles.org/) XML, a format for citation rules. The authored style uses explicit `en-US` terms, complete author lists, and a fixed bibliography sort order. Checks compare exact text and native output order. This style measures basic author-date rendering. APA fidelity, locale coverage, and disambiguation need their own cases and expectations.

RefKit's public citation request also computes a bibliography. That cost remains in `render.citation` and is recorded as `citation_api_bibliography`. These are consumer-request measurements, so library API design contributes to the result. Polars batch citation cases use the measured plugin's bundled APA style and form a separate lane.

### Upstream formatter specifications

The formatter cases come from [bibtex-tidy](https://github.com/FlamingTempura/bibtex-tidy) revision `f98c467e27a18e937c1c221caa3a6ecd760b69fb`. [The manifest](../packages/refkit-bench/src/refkit_bench/data/bibtex-tidy/cases.json) records source path, YAML document index, dimensions, license, and hashes of source, input, options, and expected output.

Run the full conformance check:

```bash
uv run --no-sync refkit-bench check --lane format.spec \
  --output packages/refkit-bench/results/formatter-conformance.json
```

The source checkout and published npm package both identify as `1.14.0`, but the checkout contains different behavior. Published `bibtex-tidy` differs on the `numeric` and `generate-keys` expectations. The report keeps those failures with both identities and the first differing line. Exit code 1 means at least one selected contract failed. Interpret it as conformance to the pinned source specifications. The published package defines its own compatibility contract.

`run` validates every selected case before collecting timings. A failure stops that run and records a failure manifest. Select an explicit shared contract such as `format.layout` for package comparisons, or a RefKit specification case for a focused regression experiment:

```bash
uv run --no-sync refkit-bench run --lane format.layout --dataset real \
  --output packages/refkit-bench/results/format-layout
uv run --no-sync refkit-bench run --lane format.spec --dataset numeric \
  --package refkit --output packages/refkit-bench/results/format-numeric
```

### Key allocation under collisions

`format.keys` uses the explicit `[year]` template. Entries sharing a year receive suffixes `a` through `z`, then `aa`, `ab`, and so on. This deliberately stresses collision allocation. Published `bibtex-tidy` 1.14.0 emits punctuation after the first 26 suffixes and fails this contract on the larger scaling inputs. The failure stays visible, while RefKit can be measured against its own baseline:

```bash
uv run --no-sync refkit-bench check --lane format.keys --dataset 1k
uv run --no-sync refkit-bench run --lane format.keys --dataset 1k \
  --package refkit --output packages/refkit-bench/results/key-allocation
```

The first command reports the participant difference with exit code 1. Smaller `tiny`, `medium`, `large`, and `real` datasets satisfy the shared contract for both participants.

## Measurement protocol

[pyperf](https://pyperf.readthedocs.io/en/latest/runner.html) calibrates loop counts and runs measurements in separate processes. Defaults are 10 workers, 10 warmup batches, five measured batches per worker, and a 100 ms target per batch. Calibration and warmup observations remain in the pyperf archive.

1. Validate each selected case in its own subprocess and retain every outcome.
2. Shuffle case order with a generated, recorded seed. `--seed` reproduces an order.
3. Prepare each measurement worker independently with the preflight environment and validate once before warmup.
4. Time a calibrated batch of complete public operations.
5. Validate the batch's final returned value after stopping the timer.
6. Verify the worker's artifact fingerprint, case contract, and benchmark-source fingerprint against preflight before accepting samples.

Python's cyclic garbage collector stays enabled. The Node participant uses a persistent child per worker and V8's normal garbage collection. Node times the public `tidy` calls with its own monotonic clock. Startup, JSON transport, and validation stay outside that clock. Runtime, V8, package, and artifact identities are recorded. Fixed warmup counts do not establish that a JIT compiler has stabilized, so inspect the retained samples when comparing small effects.

Cases run sequentially, with worker runs grouped by case. Run on an idle, plugged-in machine and record power or CPU tuning choices alongside the output. On systems with CPU affinity, workers inherit the caller's CPU selection by default. `--affinity 0,2-3` selects CPUs explicitly before libraries, threads, or formatter children start. The harness records the effective selection and verifies process and Linux thread masks after setup and each batch, outside the timer. Unsupported explicit selections fail during preflight.

Timeouts terminate the benchmark process tree, including formatter children. On Windows, the parent process bounds each case at `(processes + 6) × timeout` seconds. On other platforms, pyperf also enforces the per-worker timeout. Interrupted runs also clean up their workers.

## Compare a change

Keep the benchmark source and selected cases fixed. Rebuild the candidate native adapters, then run the same command into a new directory:

```bash
make refkit-develop-release polars-refkit-develop-release
uv run --no-sync refkit-bench run --lane parse.bibtex --dataset real \
  --output packages/refkit-bench/results/candidate
uv run --no-sync refkit-bench compare \
  packages/refkit-bench/results/baseline \
  packages/refkit-bench/results/candidate
```

The ratio is **candidate / baseline**, so values below 1 indicate lower elapsed time. Each case is reported separately. The summary statistic is the median of worker means. This gives each independent worker equal weight, including when workers contain different numbers of samples.

With at least five measured workers in each run, `compare` reports a 95% percentile bootstrap interval by resampling workers within each run. These are exploratory per-case intervals with no multiple-comparison correction. An interval excluding 1 does not rule out thermal drift, background load, or systematic ordering effects. Repeat close results in alternating baseline/candidate sessions, such as A/B/A or ABBA, and inspect worker variation before attributing a change to RefKit.

Comparison requires matching benchmark definitions, case sets, inputs, options, style contracts, runtime settings, host/CPU identity, and observed affinity. Package artifact identities may change, since those are the implementations being compared. A changed benchmark definition or fixture requires a new baseline.

## Inspect and retain evidence

`manifest.json` records selection order, configuration, validation outcomes, setup time, environment, input provenance, and artifact identity. `timings.json` uses pyperf's format and is bound to the manifest by SHA-256. Native adapter fingerprints include their Python wrappers. Polars fingerprints also include the host Python package and runtime binary. Installed source revision remains `unknown` when package metadata cannot establish it. The runner's checkout revision identifies the harness separately.

```bash
uv run --no-sync refkit-bench report packages/refkit-bench/results/candidate --json
uv run --no-sync python -m pyperf stats packages/refkit-bench/results/candidate/timings.json
uv run --no-sync python -m pyperf check packages/refkit-bench/results/candidate/timings.json
```

Keep the full result directories and exact commands. Result files stay local under `packages/refkit-bench/results/`. Performance claims need a named case, implementation identities, machine, raw samples, uncertainty, and repeated-session evidence. Correctness checks run in CI, together with a short native/Node runner smoke test. The Benchmarks workflow adds advisory performance comparisons on hosted runners.

## Extend the suite

Add a lane when it answers a distinct consumer question. Define its input shape, unit of work, setup boundary, exact output requirements, and eligible participants before collecting timings. Add independently sourced expectations and the smallest test that rejects a plausible incorrect result. Register the case, then run `make benchmark-test` and the case's `check` command.

Factories return `Prepared` operations. Keep package-specific conversion and setup in `parsing.py`, `rendering.py`, `formatting.py`, or `tabular.py`. Keep timing, process lifecycle, provenance, and result analysis in their respective modules. Runtime packages depend inward on the capability core and never import benchmark code.


## Main branch and release benchmarks

The `Benchmarks` [GitHub Actions](https://docs.github.com/en/actions) workflow finds or measures release-build performance on each push to `main`. The release workflow calls the same workflow. A lightweight planning job fingerprints the inputs, then a shared result workflow checks for matching evidence before starting Linux, Windows, and macOS runners.

The input fingerprint covers tracked core and adapter sources, runtime resources, Cargo and Python manifests and locks, benchmark code and fixtures, build settings, and measurement workflow configuration. Git paths, modes, and blob identities make additions, moves, deletions, and content changes part of the identity. Documentation, instructions, and standalone tests sit outside that set. Version changes count because native adapters expose compiled package versions.

Complete artifacts with the same fingerprint can be reused from trusted main or release runs. Concurrent requests for the same fingerprint [queue in GitHub Actions](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#concurrency), so a release can reuse the main run that is already measuring its code. Distinct fingerprints can run concurrently. If matching evidence is unavailable, changed inputs and release requests collect fresh measurements. An unchanged main commit skips measurement and reports that saved timings are unavailable.

A docs-only commit skips measurement, reuses saved candidate timings when available, and reports unchanged inputs relative to its parent. Reused reports keep their original measured commits, native artifact hashes, compiler, and runner environment. This is equality of recorded source, build, dependency, and benchmark inputs. It does not assert byte equality with a newly rebuilt release wheel. Changed main inputs reuse a comparison when both candidate and baseline fingerprints match. Other transitions receive fresh measurements on one runner. A release can reuse candidate evidence from a different transition, presented as absolute timings.

### Measurement and comparison

For a new measurement, the baseline is the previous `main` tip and the candidate is the new tip. A release tag uses its commit's first parent. A push containing several commits measures their combined effect. Each OS builds both revisions in release mode, then measures them sequentially on the same runner. The three OS jobs run in parallel, independently of distribution and documentation checks. Rust compilation and Python downloads are cached.

Each revision uses separate Cargo target and intermediate build directories beneath the configured `CARGO_TARGET_DIR` root. [Cargo build state](https://doc.rust-lang.org/cargo/reference/build-cache.html) tracks relative source paths and modification times, so these directories belong to one revision. Dependency download caches remain shared.

Both revisions use the candidate's benchmark harness and locked Python dependencies in separate environments. The comparison isolates RefKit's Python and native implementation changes under that dependency set. Changes to dependency versions need a separate experiment using each revision's dependencies.

The CI selection is `--lane all --dataset real --package refkit --package polars-eager --package polars-lazy`. It covers 14 cases across parsing, inspection, editing, rendering, formatting, and eager/lazy Polars expressions. Each case uses five workers, five warmup batches, five measured batches, and a 50 ms calibration target. Case order uses seed 2026. Revision order alternates with the workflow run number.

| Result | 95% candidate/base interval |
| --- | --- |
| Faster | Entire interval below 0.95. |
| Slower | Entire interval above 1.05. |
| Same | Entire interval inside 0.95 to 1.05. |
| Inconclusive | Interval overlaps a boundary or spans both directions. |

The percent change is `(candidate / baseline - 1) × 100`. Negative values mean lower elapsed time. Same describes the 5% band. Hosted-runner noise, thermal drift, and background load still apply, so confirm close changes with repeated controlled runs. Timing regressions remain advisory. Measurement and conformance failures fail the workflow.

### Compare a selected baseline

Use the [GitHub CLI](https://cli.github.com/manual/gh_workflow_run) to measure main against a full baseline commit SHA:

```bash
gh workflow run benchmarks.yml --ref main -f baseline="$(git rev-parse HEAD^)"
```

The dispatched run uses the workflow and benchmark harness from main. Its report records the selected baseline and current candidate commits, and cached comparisons must match both input fingerprints.

### Artifacts, commit comments, and releases

The `Consolidated results` job writes tables to the [job summary](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-commands#adding-a-job-summary) and uploads `benchmark-results-<fingerprint>` with:

| File | Contents |
| --- | --- |
| `report.json` | Schema 1, input fingerprint, measured commits, current request and reuse status, the 5% comparison band, and per-platform comparisons. |
| `summary.md` | The overview and tables shown in the job summary. Reused evidence shows saved candidate timings. |
| `<OS>/comparison.json` | Revision identities, execution order, runner image, elapsed times, worker counts, ratios, and intervals. |
| `<OS>/baseline/*.json`, `<OS>/candidate/*.json` | Validation manifests, artifact fingerprints, environment metadata, and raw pyperf samples. |

The consolidated artifact is retained for 30 days. Per-platform transfer artifacts expire after one day. Failed-job reruns reuse successful platform artifacts while they remain available. Reuse validates coverage and raw timing hashes. Missing measurements appear as failed platforms.

`Benchmark commit comment` updates one bot comment on the measured main commit with the tables and a workflow link. It runs trusted default-branch code with permission to write commit comments. Benchmark jobs have read access. The publisher reads the archive as data and checks the repository, requested commit, and latest run attempt before posting. Reruns update the comment on the same commit. GitHub requires the publisher workflow to be on the default branch before it can run.

The release-complete check waits for benchmark evidence and both package publishers. After creating release notes, the release workflow attaches `benchmark-results.json`, `benchmark-results.md`, and `benchmark-results.zip`. The ZIP contains the full evidence directory. Packaging verifies that the fingerprint and requested commit match the release and that raw timing hashes cover all three platforms.
