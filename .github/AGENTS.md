# CI And Release Workflows

GitHub Actions builds and tests source, CPython distributions, and PyEmscripten distributions before publication.

- Pin third-party actions to full commit SHAs and keep `persist-credentials: false` on checkout steps.
- Reuse `workflows/source-checks.yml` for Python, Rust, MSRV, and repository contracts.
- Check the Windows benchmark worker protocol and process deadline with the Python participant. Benchmark tests use their package-local Python and Node dependencies. The source-check workflow runs capability tests and a smoke measurement, with timing thresholds reserved for controlled hosts.
- Reuse `workflows/artifacts-refkit.yml` and `workflows/artifacts-polars-refkit.yml` from CI and publication. Keep build jobs separate from installed-artifact tests so failures identify the affected boundary.
- Configure Rust path remapping before native compilation. Normalize wheels and run the distribution contract before upload.
- Resolve Pyodide toolchains from `.github/pyodide/runtime.json` and the pinned xbuild environment.
- Keep workflows outside product semantics. Compose Make targets, build distributions, install them in clean environments, and report failures at the affected package boundary.
- Publish `refkit` and `polars-refkit` after their own artifact tests, then join both packages at release completion.
- `workflows/docs.yml` validates the root and `/refkit/` documentation builds. `workflows/ci.yml` deploys the validated Pages artifact on main.
- Attach each adapter's Agent Plugin to direct Maturin wheel output before normalization, archive validation, artifact upload, or installed-runtime tests.
- Record source revision, tool versions, build constraints, and archive hashes with each artifact. Verify the complete package artifact set before publication.
- Validate the complete archive set on Linux before each artifact workflow succeeds. Keep text checkout bytes at LF through `.gitattributes` so packaged resources match across operating systems.
- Run shared installed probes from the installed `refkit-tests` wheel on CPython, Pyodide, and rebuilt sdists. Agent recipe checks read the installed skill resources. `test-support.yml` builds shared support once per run, and candidate probes execute outside the checkout.
- Build CPython wheels with the Python 3.10 stable ABI. Test source, installed wheels, and rebuilt sdists on Python 3.10, and retain Python 3.14 runtime coverage.
- Treat tags and publish jobs as external state changes that require explicit authorization.

Run `actionlint .github/workflows/*.yml` and the affected package checks. See [packaging and release](../development_docs/packaging-and-release.md) for the artifact contract.
