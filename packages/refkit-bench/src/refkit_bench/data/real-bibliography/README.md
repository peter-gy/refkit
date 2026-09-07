# Curated Bibliography Fixture

`records.json` supplies the complete author lists and bibliographic fields for the `real` workload. The runner generates BibTeX and CSL JSON from these same records. Synthetic raw workloads supply comments, string declarations, preambles, duplicate fields, and malformed blocks.

Each record names its DBLP catalogue URL. [DBLP](https://dblp.org/) curates computer-science publication metadata and [releases its metadata under CC0 1.0](https://dblp.org/db/about/copyright.html). Result rows identify the source license as `CC0-1.0` and hash the actual generated input.

The source trail was reviewed on 2026-09-07. Ten source URLs were recovered from `biburl` fields in the repository's bibliography. The Chen and Liu catalogue records were identified through DBLP search. `source_status` records this distinction. Direct BibTeX downloads encountered DBLP's browser challenge, so the fixture records catalogue identification rather than a fresh download or an archive extraction date. The Liu record uses the catalogue's 2020 issue date and article-page range.

To reproduce an input, load `real` with `materialize_workload("real", directory)`. To refresh the metadata, review the linked catalogue records, update complete author lists and fields in `records.json`, and run `make benchmark-test`. Tests compare every generated record's authors, title, type, year, container, volume, pages, and DOI through an independent parser before timing comparisons are accepted.
