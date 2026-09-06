# Python Adapter

`refkit` is the Python adapter and public Python composition root over `crates/refkit-core`.

- Depend inward through the public `refkit-core` Rust API. Keep Hayagriva, BibLaTeX, Polars, and benchmark code outside this package.
- Own path reads and writes, extension detection, Python-facing diagnostics, PyO3 conversion, exceptions, and GIL policy.
- Register native objects through `refkit._native`, then expose the supported Python API from `src/refkit/__init__.py` and `src/refkit/__init__.pyi`.
- When the core lacks an operation, add a typed core capability and call it. Keep bibliography semantics out of PyO3 code and Python helpers.
- Release the GIL around parsing, rendering, formatting, and filesystem work that stays independent of Python objects.
- Keep unsendable raw document state on the GIL-bound path.
- Update `_native.pyi`, package exports, public docs, and boundary tests with every public change.

Rebuild with `(cd packages/refkit && uv run maturin develop)`. Run `make python-lint typecheck test rust` from the repository root.

Wheel changes need CPython and PyEmscripten installation evidence. Package metadata keeps the extension at `refkit._native`.
