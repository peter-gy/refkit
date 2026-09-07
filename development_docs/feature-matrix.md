# Bibliography Package Boundaries

Choose comparison packages by the workflow they own. RefKit exposes normalized parsing and rendering through `Library` and `Document`, raw editing through `BibDocument`, and column operations through `pl.Expr.refkit`. The [capability map](capabilities.md) and public references describe the current RefKit contract.

## Reference source set

The comparison observations came from these source revisions, inspected on 2026-06-10. Refresh the complete relevant source path before making a current upstream compatibility claim.

| Project | Repository | Inspected revision |
| --- | --- | --- |
| citeproc-js | <https://github.com/juris-m/citeproc-js> | `6f5f579771261375cddd3c3f15a5152091d786db` |
| citeproc-py | <https://github.com/citeproc-py/citeproc-py> | `5085ce96b7cceeaed5e4ac9f44f2a2e1a57fa1df` |
| python-bibtexparser | <https://github.com/sciunto-org/python-bibtexparser> | `b0f77f9828737af760442293e13ebf5f87827042` |

The benchmark package records the installed comparison versions separately. A source-research revision and a benchmark participant version can differ.

## Choose the comparison boundary

| Workflow | Reference package boundary | RefKit boundary |
| --- | --- | --- |
| Process citations supplied by a document editor | citeproc-js accepts host-retrieved item data and ordered citation updates. | `Document.render` consumes one complete ordered citation sequence with optional note numbers. |
| Render prepared references through a CSL style | citeproc-py combines source records, style, bibliography registration, and output formatting. | `Library`, `Style`, and `Document` produce citations plus a cited bibliography. |
| Inspect and transform raw BibTeX blocks | python-bibtexparser exposes blocks, fields, writers, and transformation middleware. | `BibDocument` exposes source occurrences and existing-field edits. `tidy_bibtex` owns whole-document formatting. |
| Build a custom output consumer | citeproc-js and citeproc-py expose formatter-oriented output. | `Rendered.tree` and `layout` retain typed nodes, source identity, and bibliography layout. |
| Process bibliography columns | Compare the complete package workflow including any Python materialization. | Polars `entries`, `cite`, `cite_each`, `cite_group`, and `full_bibliography` execute inside eager/lazy query plans. |

RefKit rendering expressions select `output="text"`, `"html"`, or `"rendered"`. Report expressions retain structured failures. These host contracts should be compared with an equivalent end result, including setup and conversion costs where the lane includes them.

## Boundaries to preserve

- Normalized data and raw source retain separate owners. Source repair cannot be judged solely by normalized entry counts.
- Prepared rendering excludes parsing and style setup. Path-based rendering includes those costs.
- A document sequence and a standalone citation can produce different numbering, note context, or disambiguation.
- Raw syntax preservation, field insertion/removal, and existing-field edits are separate capabilities.
- Bibliography output needs complete, equivalent author and reference records across participants.

Use the [benchmark guide](benchmarks.md) for lane definitions, evidence provenance, artifact metadata, and measurement limits. The public [rendering guide](../docs/guides/render-citations.md) and [raw-edit guide](../docs/guides/edit-bibtex.md) show complete workflows.

## Source paths for a refresh

| Project | Primary paths |
| --- | --- |
| citeproc-js | `README.rst`, `package.json`, `src/api_cite.js`, `src/api_bibliography.js`, `src/api_update.js`, `src/formats.js`, `src/state.js`, `src/system.js`, and integration fixtures. |
| citeproc-py | `README.md`, `setup.py`, `citeproc/frontend.py`, `citeproc/model.py`, `citeproc/source/`, `citeproc/formatter/`, and `tests/failing_tests.txt`. |
| python-bibtexparser | `README.md`, `bibtexparser/entrypoint.py`, `library.py`, `model.py`, `splitter.py`, `writer.py`, `middlewares/`, and parser/writer/middleware tests. |
| RefKit | Public exports and stubs, `crates/refkit-core/src`, both native adapters, installed artifact probes, and benchmark adapters. |
