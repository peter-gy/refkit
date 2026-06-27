# Real Bibliography Fixture

`references.bib` is a compact real-world BibTeX fixture derived from public source bibliographies.

The fixture keeps in-the-wild syntax that parser and renderer tests should exercise:

- top-level comments
- Unicode author and title text
- DOI, URL, arXiv, DBLP, conference, and journal fields
- month abbreviations
- BibTeX capitalization braces
- fields that some comparison libraries ignore or warn about

Use this fixture when a test or benchmark needs realistic BibTeX behavior without carrying a large source corpus. Result metadata uses the `real_bibliography_subset` workload family and `mixed-source-licenses` source license marker.
