# CI And Release Workflows

GitHub Actions builds and tests source, CPython distributions, and PyEmscripten distributions before publication.

- Pin third-party actions to full commit SHAs and keep `persist-credentials: false` on checkout steps.
- Reuse `workflows/source-checks.yml` for Python, Rust, MSRV, and repository contracts.
- Keep package build jobs separate from installed-artifact tests so failures identify the affected boundary.
- Configure Rust path remapping before native compilation. Normalize wheels and run the distribution contract before upload.
- Resolve Pyodide toolchains from `.github/pyodide/runtime.json` and the pinned xbuild environment.
- Preserve the publish dependency from `refkit-core` to `refkit`. Publish `polars-refkit` after its own artifact tests, then join all packages at release completion.
- Treat tags and publish jobs as external state changes that require explicit authorization.

Run `actionlint .github/workflows/*.yml` and the affected package checks. See [packaging and release](../development_docs/packaging-and-release.md) for the artifact contract.
