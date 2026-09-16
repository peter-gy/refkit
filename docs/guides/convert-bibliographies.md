---
description: Decode and encode BibLaTeX, Hayagriva YAML, and CSL-JSON with field-level conversion reports.
---

# Convert Bibliographies

Use `decode`, `encode`, and `convert` to move bibliography data between formats. Each operation reports fields that changed or could not be represented.

::: code-group

```python [Python]
import refkit as rk

source = "@book{doe2024, author={Doe, Jane}, title={A Book}, year={2024}}"
converted = rk.convert(source, source_format="biblatex", target_format="csl-json", loss="error")
decoded = rk.decode(converted["text"], format="csl-json", loss="error")
document = rk.Document(decoded["library"], rk.Style.load("apa"))
print(document.render([rk.Citation("intro", "doe2024")])["intro"].text)
```

```ts [TypeScript]
import * as rk from "refkit-js";

const source = "@book{doe2024, author={Doe, Jane}, title={A Book}, year={2024}}";
const converted = rk.convert(source, { sourceFormat: "biblatex", targetFormat: "csl-json", loss: "error" });
const decoded = rk.decode(converted.text, { format: "csl-json", loss: "error" });
const document = new rk.Document(decoded.library, rk.Style.load("apa"));
console.log(document.render([new rk.Citation("intro", "doe2024")]).get("intro").text);
```

:::

Both examples print `(Doe, 2024)`. TypeScript examples run in Node.js. Browser callers [initialize RefKit](/guides/browser#initialize-the-module) first.

## Choose a format

| Format name | Input and output |
| --- | --- |
| `biblatex` | BibTeX/BibLaTeX input and normalized BibLaTeX output. |
| `hayagriva` | Hayagriva bibliography YAML. |
| `csl-json` | An array of [CSL item objects](https://github.com/citation-style-language/schema/blob/master/schemas/input/csl-data.json) with `id` and `type`. |

`decode` returns a library, its source format, and conversion issues. `encode` accepts an existing library and returns target text, its format, and issues. `convert` combines both operations and returns source/target formats, text, issues, and parser diagnostics.

Use [record snapshots](/guides/parse-bibliographies#exchange-complete-records) to exchange the complete RefKit model across languages. Use [BibDocument](/guides/edit-bibtex) for source-preserving BibTeX edits. Codecs normalize bibliography data and source syntax.

## Inspect conversion loss

The default `loss="report"` returns supported output with an issue list. Use `loss="error"` to refuse reported data loss:

::: code-group

```python [Python]
range_source = '[{"id":"range","type":"book","issued":{"date-parts":[[2020],[2024]]}}]'
range_library = rk.decode(range_source, format="csl-json")["library"]
reported = rk.encode(range_library, format="hayagriva")
assert any(issue["lossy"] for issue in reported["issues"])
try:
    rk.encode(range_library, format="hayagriva", loss="error")
except rk.ConversionError as error:
    assert any(issue["path"].startswith("date") for issue in error.issues)
else:
    raise AssertionError("The date range requires a loss report")
```

```ts [TypeScript]
const rangeSource = '[{"id":"range","type":"book","issued":{"date-parts":[[2020],[2024]]}}]';
const rangeLibrary = rk.decode(rangeSource, { format: "csl-json" }).library;
const reported = rk.encode(rangeLibrary, { format: "hayagriva" });
if (!reported.issues.some((issue) => issue.lossy)) throw new Error("Missing loss report");
try {
  rk.encode(rangeLibrary, { format: "hayagriva", loss: "error" });
  throw new Error("The date range requires a loss report");
} catch (error) {
  if (!(error instanceof rk.ConversionError)) throw error;
  if (!error.issues.some((issue) => issue.path.startsWith("date"))) throw error;
}
```

:::

Hayagriva's date model represents one date, so this export uses the range's end and reports the changed date fields. Refusal leaves the input library unchanged.

Every issue has `code`, `stage`, `entry`, `path`, `lossy`, and `message`. Decode paths identify source or normalized fields. Encode paths use the canonical record schema, including snake_case names in both bindings. Informational issues, such as retained source extensions, have `lossy=false`.

Export checks decode generated text and compare structured record values and nonredundant source extensions. Original-source annotations describe provenance and are excluded from that comparison. Known type specializations and numeric/date approximations receive explicit issues. Strict mode preserves this record-data contract, while `BibDocument` owns exact source writeback.

## Supported mappings

| Capability | Contract |
| --- | --- |
| Creators | Personal and organizational names, supported roles, particles, and suffixes. Unrepresented attributes produce issues. |
| Dates | Partial dates and closed ranges in CSL-JSON and BibLaTeX. Open ranges, seasons, time, and uncertainty are checked against the target representation. |
| Text | BibLaTeX and Hayagriva retain supported protected/math chunks. CSL-JSON text is handled as literal strings, so lost chunk distinctions are reported. |
| Containers | Common article, chapter, reference-entry, and book/container fields. Hierarchy changes appear in reports. |
| Identifiers | DOI, ISBN, ISSN, and supported format-specific identifiers. Omitted schemes are reported. |
| Extensions | Source-specific fields can return to their original format. Fields unavailable in another format produce issues. |
| Language | BibLaTeX language names are mapped explicitly. Region changes or unavailable mappings produce issues. |

Unknown CSL fields are retained under `extensions["csl-json"]`. Unmodeled CSL type names use `Misc` with retained source-type metadata. This profile describes data interchange, not full CSL processor conformance.

## Keep recovery separate

For BibLaTeX input, `recovery="report"` can retain usable records after parser repairs. Conversion reports retain parser diagnostics separately from conversion issues. Combining recovery with `loss="error"` refuses dropped or literalized input. Hayagriva and CSL-JSON inputs use exact parsing.

Source and encoded bibliography text are bounded to 16 MiB. Record snapshots retain their [separate limits](/reference/data-shapes#bibliography-records).
