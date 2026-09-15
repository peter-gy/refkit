# RefKit

Parse bibliography data, render citations and bibliographies, and edit raw BibTeX from Python. RefKit accepts BibTeX, BibLaTeX, [Hayagriva YAML](https://github.com/typst/hayagriva), and CSL-JSON, with [Citation Style Language](https://citationstyles.org/) rendering into text, HTML, and structured trees. Construct libraries from structured records or [convert formats with loss reports](https://peter-gy.github.io/refkit/guides/convert-bibliographies).

`BibDocument.apply_patch` applies atomic entry and field edits and returns a new snapshot with byte changes and occurrence mappings. Existing handles retain their original values and spans. The [raw editing guide](https://peter-gy.github.io/refkit/guides/edit-bibtex) covers patch operations and reference rewrites.

[Documentation](https://peter-gy.github.io/refkit/) · [Python reference](https://peter-gy.github.io/refkit/reference/python) · [Polars](https://peter-gy.github.io/refkit/guides/polars)

## Render a citation

Install in Python 3.10 through 3.14:

```bash
python -m pip install refkit
```

```python
import refkit as rk

library = rk.Library.parse_bibtex("""
@article{doe2024,
  author = {Doe, Jane},
  title = {Fast Citations},
  journal = {Journal of Citation Tests},
  year = {2024}
}
""")
document = rk.Document(library, rk.Style.load("apa"), locale="en-US")
result = document.render([rk.Citation("intro", "doe2024")])

print(result["intro"].text)
print(result.bibliography.text)
```

```text
(Doe, 2024)
Doe, J. (2024). Fast Citations. Journal of Citation Tests.
```

Pass related citations in one ordered render call so numbering and
disambiguation share the same context. Continue with the
[Python quickstart](https://peter-gy.github.io/refkit/get-started).

## Use with agents

The `refkit` package includes an [Agent Plugin](https://agent-plugins.org/)
with version-matched task guidance. An agent that can execute Python can
discover the installed workflow:

```python
import refkit.agent

help(refkit.agent)
```

[Agent integration](https://peter-gy.github.io/refkit/reference/agent-docs)
covers discovery and packaged resources.

## Choose a workflow

- [Parse and recover](https://peter-gy.github.io/refkit/guides/parse-bibliographies) normalized entries and structured diagnostics.
- [Edit BibTeX](https://peter-gy.github.io/refkit/guides/edit-bibtex) by source occurrence while preserving surrounding text.
- [Format BibTeX](https://peter-gy.github.io/refkit/guides/format-bibtex) with key generation, duplicate handling, and rename reports.
- [Render structured output](https://peter-gy.github.io/refkit/guides/render-output) with source identity and bibliography layout.
- [Run in Pyodide](https://peter-gy.github.io/refkit/pyodide) with the tested WebAssembly runtime.

RefKit is alpha software. Public APIs may change before 1.0.

[Apache-2.0 license](https://github.com/peter-gy/refkit/blob/main/LICENSE)
