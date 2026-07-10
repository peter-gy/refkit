# Polars Interface

`polars-refkit` owns row-level bibliography expressions and a package-local Rust workspace.

- Keep `polars`, `polars-core`, `pyo3`, and `pyo3-polars` aligned as one plugin ABI family.
- Update the package-local Cargo lockfile with every Rust dependency change.
- Treat strings as column names. Use `pl.lit(...)` for literal citation keys or BibTeX input.
- Map row parse and formatting failures to null value expressions. Expose details through report and diagnostics expressions.
- Keep eager, lazy, broadcast, null, dtype, and installed-wheel behavior covered at the public expression boundary.
- Update namespace methods, top-level functions, runtime signatures, `.pyi` files, and Rust keyword records together.

Rebuild with `(cd packages/polars-refkit && uv run maturin develop)`. Run `make typecheck test rust` from the repository root.
