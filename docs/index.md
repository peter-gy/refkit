---
layout: home
title: RefKit
description: Parse, render, inspect, format, and edit bibliography data from Python, JavaScript, and Polars.

hero:
  text: Parse, cite, and tidy bibliographies.
  tagline: One Rust core. Python and TypeScript APIs for bibliography parsing, citation rendering, and source-preserving BibTeX edits.
  image:
    light: /brand/refkit-lockup-vertical-light.svg
    dark: /brand/refkit-lockup-vertical-dark.svg
    alt: RefKit
  actions:
    - theme: brand
      text: Get started
      link: /get-started
    - theme: alt
      text: How RefKit works
      link: /concepts/how-refkit-works

features:
  - icon:
      light: /icons/scan-text-light.svg
      dark: /icons/scan-text-dark.svg
      alt: ""
      width: "28"
      height: "28"
    title: Normalize bibliography data
    details: Parse BibTeX, BibLaTeX, or Hayagriva YAML into typed entries with explicit recovery diagnostics.
    link: /guides/parse-bibliographies
    linkText: Parse a bibliography
  - icon:
      light: /icons/quote-light.svg
      dark: /icons/quote-dark.svg
      alt: ""
      width: "28"
      height: "28"
    title: Render citations and bibliographies
    details: Apply Citation Style Language styles and locales to ordered citations, then read text, HTML, or a structured render tree.
    link: /guides/render-citations
    linkText: Render a document
  - icon:
      light: /icons/file-pen-line-light.svg
      dark: /icons/file-pen-line-dark.svg
      alt: ""
      width: "28"
      height: "28"
    title: Preserve raw BibTeX
    details: Inspect source-order blocks, address duplicate occurrences, edit field values, and write back the surrounding source.
    link: /guides/edit-bibtex
    linkText: Edit raw BibTeX
  - icon:
      light: /icons/table-properties-light.svg
      dark: /icons/table-properties-dark.svg
      alt: ""
      width: "28"
      height: "28"
    title: Work inside Polars
    details: Parse, inspect, tidy, and render bibliography columns in eager and lazy query plans.
    link: /guides/polars
    linkText: Use Polars expressions
---
