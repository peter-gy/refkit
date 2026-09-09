export type RecoveryPolicy = "error" | "report";
export type RawSpan = readonly [number, number];

export interface ResolvedBibEntry {
  readonly key: string;
  readonly entryType: string;
  readonly fields: Readonly<Record<string, string>>;
}

export interface Diagnostic {
  readonly code: string;
  readonly severity: "error" | "warning";
  readonly action:
    "rejected" | "dropped_block" | "dropped_field" | "literalized" | "decoded";
  readonly span: RawSpan | null;
  readonly entry: string | null;
  readonly field: string | null;
  readonly message: string;
}

export interface Entry {
  readonly key: string;
  readonly entryType: string;
  readonly title: string | null;
  readonly date: string | null;
  readonly doi: string | null;
  readonly volume: string | null;
  readonly parents: readonly Entry[];
}

export type ProjectionField =
  "key" | "entryType" | "type" | "title" | "date" | "doi" | "volume";
export interface ProjectionRow {
  key?: string;
  entryType?: string;
  type?: string;
  title?: string | null;
  date?: string | null;
  doi?: string | null;
  volume?: string | null;
}

export interface BibliographyLayout {
  readonly hangingIndent: boolean;
  readonly secondFieldAlign: "Margin" | "Flush" | null;
  readonly lineSpacing: number;
  readonly entrySpacing: number;
}

export type RenderedMeta =
  | { readonly kind: "Entry"; readonly key: string; readonly itemIndex: number }
  | { readonly kind: "Names"; readonly roles: readonly string[] }
  | { readonly kind: "Name"; readonly role: string; readonly index: number }
  | {
      readonly kind:
        | "Date"
        | "Text"
        | "Number"
        | "Label"
        | "CitationNumber"
        | "CitationLabel";
    };

export interface RenderedFormatting {
  readonly fontStyle: "Normal" | "Italic";
  readonly fontVariant: "Normal" | "SmallCaps";
  readonly fontWeight: "Normal" | "Bold" | "Light";
  readonly textDecoration: "None" | "Underline";
  readonly verticalAlign: "None" | "Baseline" | "Sup" | "Sub";
}

export interface RenderedText {
  readonly kind: "Text";
  readonly text: string;
  readonly formatting: RenderedFormatting;
}
export interface RenderedElement {
  readonly kind: "Element";
  readonly display: "Block" | "LeftMargin" | "RightInline" | "Indent" | null;
  readonly meta: RenderedMeta | null;
  readonly children: readonly RenderedNode[];
}
export interface RenderedMarkup {
  readonly kind: "Markup";
  readonly value: string;
}
export interface RenderedLink {
  readonly kind: "Link";
  readonly text: string;
  readonly url: string;
  readonly formatting: RenderedFormatting;
}
export interface RenderedTransparent {
  readonly kind: "Transparent";
  readonly citeIdx: number;
  readonly formatting: RenderedFormatting;
}
export type RenderedNode =
  | RenderedText
  | RenderedElement
  | RenderedMarkup
  | RenderedLink
  | RenderedTransparent;
export interface BibliographyEntry {
  readonly kind: "bibliography-entry";
  readonly key: string;
  readonly label: RenderedNode | null;
  readonly content: readonly RenderedNode[];
}
export type RenderedTree = readonly (RenderedNode | BibliographyEntry)[];
export interface Rendered {
  readonly text: string;
  readonly html: string;
  readonly layout: BibliographyLayout | null;
  readonly tree: RenderedTree;
}

export interface RawWhitespaceBlock {
  readonly kind: "whitespace";
  readonly span: RawSpan;
}
export interface RawCommentBlock {
  readonly kind: "comment";
  readonly raw: string;
  readonly span: RawSpan;
}
export interface RawPreambleBlock {
  readonly kind: "preamble";
  readonly value: string;
  readonly span: RawSpan;
}
export interface RawStringBlock {
  readonly kind: "string";
  readonly key: string;
  readonly value: string;
  readonly span: RawSpan;
}
export interface RawEntryBlock {
  readonly kind: "entry";
  readonly id: number;
  readonly key: string;
  readonly span: RawSpan;
}
export interface RawFailedBlock {
  readonly kind: "failed";
  readonly raw: string;
  readonly error: string;
  readonly span: RawSpan;
}
export interface RawOtherBlock {
  readonly kind: "other";
  readonly raw: string;
  readonly span: RawSpan;
}
export type RawBlock =
  | RawWhitespaceBlock
  | RawCommentBlock
  | RawPreambleBlock
  | RawStringBlock
  | RawEntryBlock
  | RawFailedBlock
  | RawOtherBlock;

export type DuplicateRule = "doi" | "key" | "abstract" | "citation";
export type MergeStrategy = "first" | "last" | "combine" | "overwrite";
export interface TidyOptions {
  omit?: readonly string[] | null;
  curly?: boolean;
  numeric?: boolean;
  months?: boolean;
  space?: number;
  tab?: boolean;
  align?: boolean | number | null;
  blankLines?: boolean;
  sort?: boolean | readonly string[] | null;
  duplicates?: readonly DuplicateRule[] | null;
  merge?: MergeStrategy | null;
  stripEnclosingBraces?: boolean;
  dropAllCaps?: boolean;
  escape?: boolean;
  sortFields?: boolean | readonly string[] | null;
  stripComments?: boolean;
  trailingCommas?: boolean;
  encodeUrls?: boolean;
  tidyComments?: boolean;
  removeEmptyFields?: boolean;
  removeDuplicateFields?: boolean;
  generateKeys?: boolean | string | null;
  maxAuthors?: number | null;
  lowercase?: boolean;
  enclosingBraces?: boolean | readonly string[] | null;
  removeBraces?: boolean | readonly string[] | null;
  wrap?: boolean | number | null;
}
export interface TidyWarning {
  readonly code: "missing_key" | "duplicate_entry";
  readonly rule: DuplicateRule | null;
  readonly message: string;
}
export interface TidyRename {
  readonly entryId: number;
  readonly oldKey: string;
  readonly newKey: string;
}
export interface TidyResult {
  readonly bibtex: string;
  readonly warnings: readonly TidyWarning[];
  readonly renames: readonly TidyRename[];
  readonly count: number;
}
