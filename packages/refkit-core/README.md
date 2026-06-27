# refkit-core

`refkit-core` is the native Rust/PyO3 package used by `refkit`.
It exposes the extension module `refkit_core._refkit_core` and the Python import package `refkit_core`.

Most projects should install `refkit`:

```bash
pip install refkit
```

`refkit` pins one exact `refkit-core` version and checks that version at import time.
Direct `refkit_core` imports are supported for native integration tests, packaging smoke tests, and low-level embedding.

```python
import refkit_core

library = refkit_core.Library.parse_bibtex(
    """
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  journal = {Journal of Citation Tests},
  year = {2024}
}
"""
)
document = refkit_core.Document(library, refkit_core.Style.load("apa"), locale="en-US")
rendered = document.render([refkit_core.Citation("intro", "doe2024")])

print(rendered["intro"].text)
```

Expected output:

```text
(Doe, 2024)
```

## Package Contract

`refkit-core` owns the native implementation for:

| Capability | Public objects |
| --- | --- |
| Read normalized bibliography data | `Library.read`, `Library.parse_bibtex`, `Library.parse_yaml` |
| Render citations | `Document.render`, `Citation`, `Cite`, `CitationGroup` |
| Render bibliographies | `Document.cited_bibliography`, `Document.full_bibliography` |
| Load CSL styles and locales | `Style.load`, `Style.from_path`, `Style.from_xml`, `Locale.load` |
| Edit raw BibTeX | `BibDocument`, `BibEntry`, `BibField` |
| Inspect rendered output | `Rendered.text`, `Rendered.html`, `Rendered.tree` |

`refkit-core` does not provide the one-call path helpers from `refkit`, such as `refkit.cite` and `refkit.full_bibliography`.

## Pyodide

Pyodide uses the same native extension package as CPython.
The release workflow builds `refkit-core` for `wasm32-unknown-emscripten` with toolchain values from `pyodide config get`, then publishes the PyEmscripten wheel alongside the CPython wheels.
The pure Python `refkit` package keeps the same exact dependency on `refkit-core`, so Pyodide resolves the matching wheel from PyPI.
The current Pyodide lane targets Python 3.14 and the `pyemscripten_2026_0_wasm32` ABI.

## Development

```bash
uv sync --all-packages --group dev
uv run maturin develop --manifest-path packages/refkit-core/Cargo.toml
uv run pytest packages/refkit/tests --no-cov
```

Build only the native package:

```bash
uv build --package refkit-core --no-create-gitignore
```

## License

`refkit-core` is licensed under the Apache License, Version 2.0, available in [LICENSE](LICENSE).
