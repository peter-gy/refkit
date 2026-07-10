# Native Python Adapter

`refkit-core` exposes the portable core as `refkit_core._refkit_core`.

- Keep PyO3 classes, exceptions, Python value conversion, and GIL management in this package.
- Release the GIL around core parsing, rendering, formatting, and file work that stays independent of Python objects.
- Keep unsendable raw document state on the GIL-bound path.
- Update `src/refkit_core/_refkit_core.pyi`, package exports, and facade stubs with every public native change.
- Rebuild with `(cd packages/refkit-core && uv run maturin develop)` before running Python tests.

Wheel changes need CPython and PyEmscripten installation evidence. Package metadata must keep the extension at `refkit_core._refkit_core`.
