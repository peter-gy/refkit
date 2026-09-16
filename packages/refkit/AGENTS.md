# Python Adapter

`refkit` is the Python adapter and public Python composition root over `crates/refkit-core`.

- Depend inward through the public `refkit-core` Rust API. Keep Hayagriva, BibLaTeX, Polars, and benchmark code outside this package.
- Own path reads and writes, extension detection, Python-facing diagnostics, PyO3 conversion, exceptions, and GIL policy.
- Register native objects through `refkit._native`, then expose the supported Python API from `src/refkit/__init__.py` and `src/refkit/__init__.pyi`.
- When the core lacks an operation, add a typed core capability and call it. Keep bibliography semantics out of PyO3 code and Python helpers.
- Release the GIL around parsing, rendering, formatting, and filesystem work that stays independent of Python objects.
- Raw handles retain immutable snapshots through `Arc`. Keep them readable across Python threads and release the GIL for patch planning and application.
- Keep public dictionary and tree contracts in `refkit.types`. Native conversion, stubs, and consumer annotation tests must use the same field names and finite vocabularies.
- Normalized entries are detached `refkit.types.Entry` dictionaries. Construction and extraction share this schema. Canonical JSON snapshots use the core's versioned field names across languages.
- Update `_native.pyi`, package exports, public docs, and boundary tests with every public change.
- Keep `refkit.agent` lazy and limited to dynamic help plus packaged-resource access over the public Python API.
- Keep each packaged task example independently executable. Validate installed resources with `python -m refkit_tests.agent_examples` through editable and built-wheel boundaries.
- Keep annotation-only consumer examples in `tests/typing_samples.py`, covered by both type checkers. Test formatter keyword acceptance through actual transformations.

Rebuild with `make refkit-develop` from the repository root so the editable native module and Agent Plugin marker come from the same backend. Run `make python-lint typecheck test rust` after the rebuild.

Wheel changes need CPython and PyEmscripten installation evidence. Package metadata keeps the extension at `refkit._native`.

The `abi3` Cargo feature selects Python 3.10 stable-ABI support. Maturin owns extension linking through `PYO3_BUILD_EXTENSION_MODULE`. Keep Cargo all-feature tests linked against the selected Python development library.
