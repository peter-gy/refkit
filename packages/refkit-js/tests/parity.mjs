import { readFileSync } from "node:fs";
import {
  BibDocument,
  Citation,
  CitationGroup,
  Cite,
  Document,
  Library,
  ParseError,
  Style,
  tidyBibtex,
} from "refkit-js";

function entry(value) {
  return {
    key: value.key,
    entry_type: value.entryType,
    title: value.title,
    date: value.date,
    doi: value.doi,
    volume: value.volume,
    parents: value.parents.map(entry),
  };
}

function formatting(value) {
  return {
    font_style: value.fontStyle,
    font_variant: value.fontVariant,
    font_weight: value.fontWeight,
    text_decoration: value.textDecoration,
    vertical_align: value.verticalAlign,
  };
}

function node(value) {
  switch (value.kind) {
    case "Text":
    case "Link":
      return { ...value, formatting: formatting(value.formatting) };
    case "Transparent":
      return {
        kind: value.kind,
        cite_idx: value.citeIdx,
        formatting: formatting(value.formatting),
      };
    case "Element":
      return {
        ...value,
        meta:
          value.meta?.kind === "Entry"
            ? {
                kind: "Entry",
                key: value.meta.key,
                item_index: value.meta.itemIndex,
              }
            : value.meta,
        children: value.children.map(node),
      };
    case "bibliography-entry":
      return {
        ...value,
        label: value.label === null ? null : node(value.label),
        content: value.content.map(node),
      };
    case "Markup":
      return value;
    default:
      throw new Error(`Unexpected rendered node ${value.kind}`);
  }
}

function rendered(value) {
  return {
    text: value.text,
    html: value.html,
    tree: value.tree.map(node),
    layout:
      value.layout === null
        ? null
        : {
            hanging_indent: value.layout.hangingIndent,
            second_field_align: value.layout.secondFieldAlign,
            line_spacing: value.layout.lineSpacing,
            entry_spacing: value.layout.entrySpacing,
          },
  };
}

function renderedDocument(value) {
  return {
    citation_order: value.citationOrder,
    citations: Object.fromEntries(
      value.citationOrder.map((key) => [key, rendered(value.get(key))]),
    ),
    bibliography: rendered(value.bibliography),
  };
}

function libraryCase(input) {
  try {
    const library =
      input.format === "yaml"
        ? Library.parseYaml(input.source)
        : Library.parseBibtex(input.source, {
            recovery: input.recovery ?? "error",
          });
    const keys = library.keys();
    const selected = keys.length ? [keys.at(-1), keys[0], keys.at(-1)] : [];
    return {
      keys,
      records: library.values().map(entry),
      diagnostics: library.diagnostics,
      projection: library.project(),
      selected_projection: library
        .project(
          ["key", "entryType", "type", "title", "date", "doi", "volume"],
          { keys: selected },
        )
        .map(({ entryType, ...row }) => ({ ...row, entry_type: entryType })),
      selected_records: library.getMany(selected).map(entry),
      selected_by_type: library
        .select(input.selector ?? "article | book")
        .map(entry),
      missing: library.get("absent-reference"),
    };
  } catch (error) {
    if (error instanceof ParseError) {
      return { error: "ParseError", diagnostics: error.diagnostics };
    }
    throw error;
  }
}

function renderCase(input) {
  const library = Library.parseBibtex(input.source);
  const style = input.xml ? Style.fromXml(input.xml) : Style.load(input.style);
  const document = new Document(library, style, { locale: "en-US" });
  const citations = input.citations.map(
    (value) =>
      new Citation(
        value.id,
        new CitationGroup(
          value.items.map(
            (item) =>
              new Cite(item.key, {
                locator: item.locator,
                label: item.label,
              }),
          ),
        ),
        { noteNumber: value.note_number },
      ),
  );
  return {
    rendered: renderedDocument(document.render(citations)),
    cited_bibliography: rendered(document.citedBibliography(citations)),
    full_bibliography: rendered(document.fullBibliography()),
    fresh_render: renderedDocument(document.render(citations.slice(-1))),
    repeated_render: renderedDocument(document.render(citations)),
  };
}

function rawState(document) {
  let resolution;
  try {
    resolution = document.resolve().map(({ entryType, ...value }) => ({
      ...value,
      entry_type: entryType,
    }));
  } catch (error) {
    if (!(error instanceof ParseError)) throw error;
    resolution = { error: "ParseError", diagnostics: error.diagnostics };
  }
  return {
    resolution,
    bibtex: document.toBibtex(),
    diagnostics: document.diagnostics,
    comments: document.comments,
    preamble: document.preamble,
    strings: document.strings,
    failed_blocks: document.failedBlocks,
    blocks: document.blocks,
    keys: document.entries.uniqueKeys(),
    occurrence_keys: document.entries.occurrenceKeys(),
    entries: document.entries.occurrences().map((value) => ({
      key: value.key,
      kind: value.kind,
      span: value.span,
      field_keys: value.fields.uniqueKeys(),
      field_occurrence_keys: value.fields.occurrenceKeys(),
      fields: value.fields.occurrences().map((field) => ({
        name: field.name,
        value: field.value,
        span: field.span,
      })),
    })),
  };
}

function rawCase(input) {
  const document = BibDocument.parse(input.source);
  const before = rawState(document);
  for (const edit of input.edits) {
    const entry = document.entries.getAll(edit.key)[edit.entry];
    entry.fields.getAll(edit.field)[edit.occurrence].value = edit.value;
  }
  return { before, after: rawState(document) };
}

function tidyCase(input) {
  const names = {
    blank_lines: "blankLines",
    strip_enclosing_braces: "stripEnclosingBraces",
    drop_all_caps: "dropAllCaps",
    sort_fields: "sortFields",
    strip_comments: "stripComments",
    trailing_commas: "trailingCommas",
    encode_urls: "encodeUrls",
    tidy_comments: "tidyComments",
    remove_empty_fields: "removeEmptyFields",
    remove_duplicate_fields: "removeDuplicateFields",
    generate_keys: "generateKeys",
    max_authors: "maxAuthors",
    enclosing_braces: "enclosingBraces",
    remove_braces: "removeBraces",
  };
  const options = Object.fromEntries(
    Object.entries(input.options).map(([key, value]) => [
      names[key] ?? key,
      value,
    ]),
  );
  const result = tidyBibtex(input.source, { options });
  return {
    bibtex: result.bibtex,
    count: result.count,
    warnings: result.warnings,
    renames: result.renames.map((value) => ({
      entry_id: value.entryId,
      old_key: value.oldKey,
      new_key: value.newKey,
    })),
  };
}

const runners = {
  library: libraryCase,
  render: renderCase,
  raw: rawCase,
  tidy: tidyCase,
};
const inputs = JSON.parse(readFileSync(0, "utf8"));
const outputs = Object.fromEntries(
  inputs.map((input) => [input.name, runners[input.kind](input)]),
);
process.stdout.write(JSON.stringify(outputs));
