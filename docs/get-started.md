---
description: Install RefKit and render one Citation Style Language citation with its bibliography.
---

# Get Started

RefKit parses a bibliography, applies a [Citation Style Language](https://citationstyles.org/) (CSL) style, and returns a citation plus its bibliography. The first example uses the `refkit` Python package and runs on Python 3.10 through 3.14.

## Install `refkit`

Use [pip](https://pip.pypa.io/), Python's package installer:

```bash
python -m pip install refkit
```

The package installs a native extension. Pip selects a compatible wheel when one is available and builds from the source distribution on other platforms. A source build requires a Rust toolchain and Git. Cargo retrieves pinned source for the [Hayagriva bibliography engine](https://github.com/typst/hayagriva) and [Citationberg CSL model](https://github.com/typst/citationberg) from GitHub. The initial build needs network access unless both revisions are already cached.

## Render a citation

Create `first_citation.py`:

```python
import refkit as rk

source = """
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  journal = {Journal of Citation Tests},
  year = {2024}
}
"""

library = rk.Library.parse_bibtex(source)
style = rk.Style.load("apa")
document = rk.Document(library, style, locale="en-US")
rendered = document.render([rk.Citation("intro", "doe2024")])

print(rendered["intro"].text)
print(rendered.bibliography.text)
```

Run it:

```bash
python first_citation.py
```

Expected output:

```text
(Doe, 2024)
Doe, J. (2024). Fast Citations. Journal of Citation Tests.
```

The example introduces five RefKit objects:

- `Library` owns normalized bibliography entries and parser diagnostics.
- `Style` owns one prepared CSL style.
- `Document` combines a library, style, and locale for one ordered render operation.
- `Citation` gives a citation a stable result name.
- `RenderedDocument` returns named citations and the bibliography produced from them.

## Choose a bibliography model

RefKit exposes two views of bibliography data:

| Question | Use | Result |
| --- | --- | --- |
| Which entries can I select, project, or render? | `Library` | Normalized entries with consistent fields. |
| Which source blocks and duplicate occurrences must survive an edit? | `BibDocument` | Source-order BibTeX with editable field values. |

Read [Choose a Bibliography Model](/concepts/bibliography-models) before a workflow that edits source files.

## Choose a Python package

Install `polars-refkit` when bibliography source lives in [Polars](https://pola.rs/) string columns:

```bash
python -m pip install polars-refkit
```

```python
import polars as pl
import polars_refkit

source = """
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  year = {2024}
}
"""
frame = pl.DataFrame({"bibtex": [source], "key": ["doe2024"]})
result = frame.select(
    citation=pl.col("bibtex").refkit.cite("key"),
    entries=pl.col("bibtex").refkit.entry_count(),
)

print(result.to_dicts())
# [{'citation': '(Doe, 2024)', 'entries': 1}]
```

A string argument names a column in the Polars interface. Wrap a literal citation key or bibliography source with `pl.lit(...)`.

Continue with [How RefKit Works](/concepts/how-refkit-works) or go directly to [Render Citations](/guides/render-citations).
