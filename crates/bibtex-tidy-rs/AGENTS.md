# Formatter Compatibility Crate

`bibtex-tidy-rs` preserves the Rust formatter API over the implementation owned by `refkit-core`.

- Delegate parsing, options, warnings, and formatting behavior to the shared core.
- Keep the public Rust compatibility surface aligned with its README and tests.
- Update this crate's `Cargo.lock` when its dependency resolution changes.
- Add new formatter behavior to `crates/refkit-core` so Python, Polars, and Rust callers share one implementation.

Run `make rust-lint rust rust-floor` from the repository root.
