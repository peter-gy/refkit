# arXiv Wild BibTeX Fixture

`references-subset.bib` is a compact real-world BibTeX fixture derived from public arXiv source bibliographies.

The fixture keeps in-the-wild syntax that parser and renderer tests should exercise:

- top-level comments
- Unicode author and title text
- DOI, URL, arXiv, DBLP, conference, and journal fields
- month abbreviations
- BibTeX capitalization braces
- fields that some comparison libraries ignore or warn about

Use this fixture when a test or benchmark needs realistic BibTeX behavior without carrying a large source corpus. Result metadata should use the `arxiv_wild_subset` workload family and `mixed-arxiv-source-licenses` source license marker.
