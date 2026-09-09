# refkit-bench

Measure bibliography parsing, inspection, editing, rendering, formatting, and Polars queries through public package APIs. Each case has a fixed input and output contract. Validation runs before calibrated, process-isolated timing.

From the repository root:

```bash
make sync
npm ci --ignore-scripts --prefix packages/refkit-bench/node
make refkit-develop-release polars-refkit-develop-release
uv run --no-sync refkit-bench check --lane parse.bibtex --dataset real
uv run --no-sync refkit-bench run --lane parse.bibtex --dataset real \
  --output packages/refkit-bench/results/baseline
```

The result directory contains validation and provenance in `manifest.json`, plus raw [pyperf](https://pyperf.readthedocs.io/) measurements in `timings.json`. Use `refkit-bench report` to inspect one run and `refkit-bench compare` for a matched baseline and candidate.

The [benchmark guide](../../development_docs/benchmarks.md) covers lane boundaries, upstream formatter specifications, build requirements, uncertainty, and the regression workflow. Run `make benchmark-test` when changing this package.

## License

Apache-2.0, with the license in [LICENSE](LICENSE). The curated [bibtex-tidy](https://github.com/FlamingTempura/bibtex-tidy) specification inputs and expectations retain their MIT license and source provenance.
