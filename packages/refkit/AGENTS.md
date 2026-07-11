# Python Facade

`refkit` is the pure Python public package over the exact matching `refkit-core` release.

- Keep bibliography behavior in the native package or portable Rust core. This package owns helpers, re-exports, version checks, and Python call ergonomics.
- Preserve the import-time core version check and the exact `refkit-core` dependency.
- Treat one-call helpers as path-based APIs. Use `Library.parse_bibtex`, `Library.parse_yaml`, and `Document` for in-memory sources.
- Update `src/refkit/__init__.py`, `src/refkit/__init__.pyi`, package docs, and public tests together.
- Rebuild `packages/refkit-core` when the native boundary changed before running Python tests.

Run `make python-lint typecheck test` from the repository root. Read the [architecture](../../development_docs/architecture.md) and [testing guide](../../development_docs/testing.md) for cross-package changes.
