# bibtex-tidy-rs

`bibtex-tidy-rs` formats BibTeX text from Rust code.

```rust
use bibtex_tidy_rs::{tidy, TidyOptions};

let result = tidy("@ARTICLE{key,title={A paper}}\n", TidyOptions::default())?;
assert_eq!(result.count, 1);
# Ok::<(), bibtex_tidy_rs::TidyError>(())
```

The formatter reads BibTeX through the shared `refkit-core` raw syntax model.
