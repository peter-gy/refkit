# Polars Interface

`polars-refkit` owns row-level bibliography expressions and a package-local Rust workspace.

- Depend inward through the public `refkit-core` Rust API. Keep Hayagriva, BibLaTeX, the Python adapter, and benchmark code outside this package.
- Treat the Rust plugin, Python expression builders, and namespace registration as one adapter. Keep function names, keyword records, dtypes, and stubs aligned.
- Keep `polars`, `polars-core`, `pyo3`, and `pyo3-polars` aligned as one plugin ABI family.
- Update the package-local Cargo lockfile with every Rust dependency change.
- Treat strings as column names. Use `pl.lit(...)` for literal citation keys or BibTeX input.
- Map row parse and formatting failures to null value expressions. Expose details through report and diagnostics expressions.
- Keep eager, lazy, broadcast, null, dtype, and installed-wheel behavior covered at the public expression boundary.
- Keep formatter options in the JSON-serializable `TidyOptions` mapping. Resolve `output` to its fixed dtype when constructing the expression.
- Package the adapter-owned Agent Plugin through `build_backend.py`. Keep `polars_refkit.agent` lazy and its task examples executable through the installed-resource runner.
- Update namespace methods, top-level functions, runtime signatures, `.pyi` files, and Rust keyword records together.
- Reuse core semantic inputs and expected results. Keep broadcasting, lazy execution, row failures, dtypes, and plugin loading in Polars boundary tests.

Rebuild with `make polars-refkit-develop` from the repository root so the native plugin and Agent Plugin marker come from the same backend. Run `make typecheck test rust` from the repository root.
