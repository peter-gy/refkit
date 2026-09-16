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
  convert,
  decode,
  ConversionError,
  tidyBibtex,
} from "refkit-js";

function entry(value) {
  return JSON.parse(Library.fromRecords([value]).toJson()).records[0];
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
        .project(["key", "entryType", "title", "date", "doi", "volume"], {
          keys: selected,
        })
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
  const style = input.xml
    ? Style.fromXml(input.xml, { parentXml: input.parent_xml })
    : Style.load(input.style);
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
                purpose: item.purpose,
              }),
          ),
        ),
        { noteNumber: value.note_number },
      ),
  );
  return {
    style_id: style.cslId,
    style_title: style.title,
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
  let document = BibDocument.parse(input.source);
  const before = rawState(document);
  for (const edit of input.edits) {
    const entry = document.entries.getAll(edit.key)[edit.entry];
    const field = entry.fields.getAll(edit.field)[edit.occurrence];
    document = document.applyPatch([
      {
        kind: "set_field",
        entryId: field.entryId,
        fieldId: field.id,
        value: edit.value,
      },
    ]).document;
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
  merge: (input) => {
    const document = BibDocument.parse(input.source);
    const names = {
      entryId: "entry_id",
      fieldId: "field_id",
      entryType: "entry_type",
      retainedId: "retained_id",
      removedIds: "removed_ids",
    };
    const canonical = (value) =>
      Array.isArray(value)
        ? value.map(canonical)
        : value && typeof value === "object"
          ? Object.fromEntries(
              Object.entries(value).map(([key, value]) => [
                names[key] ?? key,
                canonical(value),
              ]),
            )
          : value;
    const report = document.findDuplicates({ rules: input.rules });
    try {
      const fields = input.fields?.map(({ entry_id, field_id, ...choice }) => ({
        ...choice,
        ...(entry_id === undefined ? {} : { entryId: entry_id }),
        ...(field_id === undefined ? {} : { fieldId: field_id }),
      }));
      const plan = document.planMerge({
        entries: input.entries,
        retain: input.retain,
        fields,
        entryType: input.entry_type,
      });
      const updated =
        plan.patch === null ? null : document.applyPatch(plan.patch).document;
      return {
        report: canonical(report),
        plan: canonical(plan),
        source: updated?.toBibtex() ?? null,
        original: document.toBibtex(),
      };
    } catch (error) {
      if (error.name === "MergeError")
        return {
          report: canonical(report),
          error: "MergeError",
          code: error.code,
          original: document.toBibtex(),
        };
      throw error;
    }
  },
  patch: (input) => {
    const document = BibDocument.parse(input.source);
    const before = rawState(document);
    const names = {
      entry_id: "entryId",
      field_id: "fieldId",
      entry_type: "entryType",
    };
    const patch = input.patch.map((operation) =>
      Object.fromEntries(
        Object.entries(operation).map(([key, value]) => [
          names[key] ?? key,
          value,
        ]),
      ),
    );
    try {
      const result = document.applyPatch(patch);
      return {
        before,
        after: rawState(result.document),
        original_after: rawState(document),
        report: {
          changes: result.changes,
          entries: result.entries,
          warnings: result.warnings.map(({ entryId, fieldId, ...warning }) => ({
            ...warning,
            entry_id: entryId,
            field_id: fieldId,
          })),
        },
      };
    } catch (error) {
      if (error.name === "PatchError")
        return {
          error: "PatchError",
          code: error.code,
          operation: error.operation,
          original_after: rawState(document),
        };
      throw error;
    }
  },
  validation: (input) => {
    try {
      const report = input.records
        ? Library.fromJson(
            JSON.stringify({ schema_version: 1, records: input.records }),
          ).validate()
        : BibDocument.parse(input.source).validate();
      const target = ({ entryId, fieldId, ...rest }) => ({
        ...rest,
        entry_id: entryId,
        field_id: fieldId,
      });
      return {
        ...report,
        issues: report.issues.map((issue) => ({
          ...issue,
          target: target(issue.target),
          related: issue.related.map(target),
        })),
      };
    } catch (error) {
      if (error instanceof ParseError)
        return { error: "ParseError", diagnostics: error.diagnostics };
      throw error;
    }
  },
  codec: (input) => {
    try {
      const report = convert(input.source, {
        sourceFormat: input.source_format,
        targetFormat: input.target_format,
        loss: input.loss,
        recovery: input.recovery,
      });
      const restored = decode(report.text, { format: input.target_format });
      return {
        report: {
          source_format: report.sourceFormat,
          target_format: report.targetFormat,
          text: report.text,
          issues: report.issues,
          diagnostics: report.diagnostics,
        },
        records: JSON.parse(restored.library.toJson()).records,
        issues: restored.issues,
      };
    } catch (error) {
      if (error instanceof ConversionError)
        return {
          error: "ConversionError",
          issues: error.issues,
          diagnostics: error.diagnostics,
        };
      throw error;
    }
  },
  library: libraryCase,
  render: renderCase,
  raw: rawCase,
  tidy: tidyCase,
  records: (input) => {
    const library = Library.fromJson(
      JSON.stringify({ schema_version: 1, records: input.records }),
    );
    const restored = Library.fromRecords(library.toRecords());
    return {
      records: JSON.parse(restored.toJson()).records,
      snapshot: restored.toJson(),
      projection: restored.project(),
      rendered: rendered(
        new Document(restored, Style.load("apa")).fullBibliography(),
      ),
    };
  },
};
const inputs = JSON.parse(readFileSync(0, "utf8"));
const outputs = Object.fromEntries(
  inputs.map((input) => [input.name, runners[input.kind](input)]),
);
process.stdout.write(JSON.stringify(outputs));
