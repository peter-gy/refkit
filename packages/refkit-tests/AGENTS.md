# Installed-Artifact Verification

`refkit_tests` owns shared agent recipe execution, installed Python and Polars probes, and runtime tests used on CPython and Pyodide.

- Keep this package outward from runtime adapters. Select an adapter by importing its probe or runtime test module. Package installation must preserve independent adapter availability.
- Use public adapter APIs and the candidate distribution's installed Agent Plugin resources. Keep bibliography semantics in the portable core.
- Run probes with `python -m refkit_tests.smoke_refkit` or `python -m refkit_tests.smoke_polars_refkit`.
- Run focused installed runtime tests from a temporary working directory with `python -m pytest --rootdir "$PWD" -o addopts= -p no:cacheprovider --pyargs refkit_tests.test_refkit_runtime` or `refkit_tests.test_polars_refkit_runtime`.
- Run authored README and guide examples with `python -m refkit_tests.check_examples --root /path/to/refkit`. The verifier reads Markdown from that checkout and executes each example against installed adapters in a temporary directory.
- Build the support wheel through `uv build --package refkit-tests --wheel`. CI installs it alongside candidate distributions in clean environments and executes probes outside the checkout.
- Keep the package within the root Ruff, ty, pyrefly, and pytest configuration. Add typing information with shared verification APIs.

Run `make python-lint typecheck test` after changes. Installed-artifact claims require a built candidate wheel and an isolated runtime probe.
