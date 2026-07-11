# refkit-core

`refkit-core` is the native Rust/PyO3 package used by `refkit`.
It exposes the extension module `refkit_core._refkit_core` and the Python import package `refkit_core`.

Most projects should install `refkit`:

```bash
pip install refkit
```

`refkit` pins one exact `refkit-core` version and checks that version at import time.
Direct `refkit_core` imports are supported for low-level integrations that use native objects without the `refkit` helper functions.

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

`tidy_bibtex` formats BibTeX text through the same native package:

```python
result = refkit_core.tidy_bibtex(
    "@ARTICLE{doe2024, pages={6-13}, year={2024}}\n",
    options=refkit_core.TidyOptions(sort_fields=True),
)

print(result.bibtex)
```

## Package Contract

`refkit-core` owns the native implementation for:

| Capability | Public objects |
| --- | --- |
| Read normalized bibliography data | `Library.read`, `Library.parse_bibtex`, `Library.parse_yaml` |
| Render citations | `Document.render`, `Citation`, `Cite`, `CitationGroup` |
| Render bibliographies | `Document.cited_bibliography`, `Document.full_bibliography` |
| Load CSL styles and locales | `Style.load`, `Style.from_path`, `Style.from_xml`, `Locale.load` |
| Format BibTeX | `tidy_bibtex`, `TidyOptions`, `TidyResult`, `TidyWarning` |
| Edit raw BibTeX | `BibDocument`, `BibEntry`, `BibField` |
| Inspect rendered output | `Rendered.text`, `Rendered.html`, `Rendered.tree` |

Use `refkit` for one-call path helpers such as `refkit.cite` and `refkit.full_bibliography`.

## Pyodide

Pyodide uses the same `refkit_core` import package as CPython. Install `refkit` to select the matching PyEmscripten wheel for Pyodide 314.0.2:

```python
import micropip

await micropip.install("refkit")
```

The Pyodide CLI also accepts `python -m pip install refkit` inside its virtual environment.

## License

`refkit-core` is licensed under the Apache License, Version 2.0, available in [LICENSE](LICENSE). See [NOTICE](NOTICE) for upstream citation and bibliography component acknowledgements.
