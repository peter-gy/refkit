export type RecoveryPolicy = "error" | "report";
import type { Library } from "./library.js";
import type { BibDocument } from "./raw.js";

export type BibliographyFormat = "biblatex" | "hayagriva" | "csl-json";
export type LossPolicy = "error" | "report";
export interface ConversionIssue {
  readonly code: string;
  readonly stage: "decode" | "encode";
  readonly entry: string | null;
  readonly path: string;
  readonly lossy: boolean;
  readonly message: string;
}
export interface DecodeReport {
  readonly library: Library;
  readonly format: BibliographyFormat;
  readonly issues: readonly ConversionIssue[];
}
export interface EncodeReport {
  readonly format: BibliographyFormat;
  readonly text: string;
  readonly issues: readonly ConversionIssue[];
}
export interface ConversionReport {
  readonly sourceFormat: BibliographyFormat;
  readonly targetFormat: BibliographyFormat;
  readonly text: string;
  readonly issues: readonly ConversionIssue[];
  readonly diagnostics: readonly Diagnostic[];
}
export type CitePurpose = "normal" | "author" | "year" | "full" | "prose";
export interface StyleMetadata {
  readonly name: string;
  readonly aliases: readonly string[];
  readonly title: string;
  readonly cslId: string;
}
export type RawSpan = readonly [number, number];

export interface RawEntryInfo {
  readonly id: number;
  readonly key: string;
  readonly kind: string;
  readonly span: RawSpan;
}
export interface RawFieldInfo {
  readonly id: number;
  readonly name: string;
  readonly value: string;
  readonly span: RawSpan;
}
export interface BibFieldValue {
  readonly name: string;
  readonly value: string;
  readonly expression?: boolean;
}
export type BibEdit =
  | {
      readonly kind: "set_field";
      readonly entryId: number;
      readonly fieldId: number;
      readonly value: string;
      readonly expression?: boolean;
    }
  | {
      readonly kind: "add_field";
      readonly entryId: number;
      readonly name: string;
      readonly value: string;
      readonly expression?: boolean;
    }
  | {
      readonly kind: "remove_field";
      readonly entryId: number;
      readonly fieldId: number;
    }
  | {
      readonly kind: "add_entry";
      readonly key: string;
      readonly entryType: string;
      readonly fields?: readonly BibFieldValue[];
      readonly before?: number | null;
    }
  | { readonly kind: "remove_entry"; readonly entryId: number }
  | {
      readonly kind: "rename_entry";
      readonly entryId: number;
      readonly key: string;
    }
  | {
      readonly kind: "set_entry_type";
      readonly entryId: number;
      readonly entryType: string;
    };
export type BibPatch = readonly BibEdit[];
export type BibPatchErrorCode =
  | "invalid_target"
  | "invalid_value"
  | "overlap"
  | "invalid_result"
  | "ambiguous_reference"
  | "reference_error"
  | "resource_limit";
export interface BibPatchChange {
  readonly operations: readonly number[];
  readonly kind: BibEdit["kind"] | "rewrite_reference";
  readonly before: RawSpan;
  readonly after: RawSpan;
}
export interface BibFieldMapping {
  readonly before: RawFieldInfo | null;
  readonly after: RawFieldInfo | null;
}
export interface BibEntryMapping {
  readonly before: RawEntryInfo | null;
  readonly after: RawEntryInfo | null;
  readonly fields: readonly BibFieldMapping[];
}
export interface BibPatchWarning {
  readonly code: "duplicate_entry" | "duplicate_field";
  readonly entryId: number;
  readonly fieldId: number | null;
  readonly message: string;
}
export interface BibPatchResult {
  readonly document: BibDocument;
  readonly changes: readonly BibPatchChange[];
  readonly entries: readonly BibEntryMapping[];
  readonly warnings: readonly BibPatchWarning[];
}

export interface DuplicateMember {
  readonly entryId: number;
  readonly key: string;
}
export interface DuplicateEvidence {
  readonly rule: DuplicateRule;
  readonly signature: string;
  readonly members: readonly number[];
}
export interface DuplicateValue {
  readonly entryId: number;
  readonly fieldId: number | null;
  readonly value: string;
  readonly expression: string;
}
export interface DuplicateConflict {
  readonly kind: "field" | "identifier" | "entry_type";
  readonly field: string;
  readonly values: readonly DuplicateValue[];
}
export interface DuplicateGroup {
  readonly id: number;
  readonly members: readonly DuplicateMember[];
  readonly evidence: readonly DuplicateEvidence[];
  readonly conflicts: readonly DuplicateConflict[];
}
export interface DuplicateReport {
  readonly rules: readonly DuplicateRule[];
  readonly groups: readonly DuplicateGroup[];
}
export type MergeFieldChoice =
  | {
      readonly kind: "take";
      readonly name: string;
      readonly entryId: number;
      readonly fieldId: number;
    }
  | { readonly kind: "drop"; readonly name: string };
export type MergeErrorCode =
  | "invalid_selection"
  | "invalid_choice"
  | "ambiguous_reference"
  | "reference_error"
  | "reference_cycle"
  | "resource_limit";
export interface MergePlan {
  readonly retainedId: number;
  readonly removedIds: readonly number[];
  readonly patch: BibPatch | null;
  readonly conflicts: readonly DuplicateConflict[];
}

export type ValidationCode =
  | "missing_required_field"
  | "superfluous_field"
  | "malformed_field"
  | "invalid_identifier"
  | "identifier_form"
  | "shared_identifier"
  | "invalid_url"
  | "empty_title"
  | "empty_name"
  | "reversed_date_range"
  | "unresolved_reference"
  | "incomplete_container";
export interface ValidationTarget {
  readonly entry: string;
  /** Canonical snake_case record path, or BibLaTeX source field name. */
  readonly path: string;
  readonly entryId: number | null;
  readonly fieldId: number | null;
  readonly span: RawSpan | null;
}
export interface ValidationIssue {
  readonly code: ValidationCode;
  readonly severity: "error" | "warning";
  readonly target: ValidationTarget;
  readonly related: readonly ValidationTarget[];
  readonly message: string;
  readonly suggestion: string | null;
}
export interface ValidationReport {
  readonly profile: "records" | "biblatex";
  readonly valid: boolean;
  readonly issues: readonly ValidationIssue[];
}

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
  readonly title?: Text | null;
  readonly authors?: readonly Name[];
  readonly editors?: readonly Name[];
  readonly affiliated?: readonly Contributors[];
  readonly date?: BibliographyDate | null;
  readonly eventDate?: BibliographyDate | null;
  readonly originalDate?: BibliographyDate | null;
  readonly publisher?: Publisher | null;
  readonly location?: Text | null;
  readonly organization?: Text | null;
  readonly issue?: ScalarValue | null;
  readonly chapter?: ScalarValue | null;
  readonly volume?: ScalarValue | null;
  readonly volumeTotal?: ScalarValue | null;
  readonly edition?: ScalarValue | null;
  readonly pageRange?: ScalarValue | null;
  readonly pageTotal?: ScalarValue | null;
  readonly timeRange?: ScalarValue | null;
  readonly runtime?: ScalarValue | null;
  readonly url?: BibliographyUrl | null;
  readonly identifiers?: Readonly<Record<string, string>>;
  readonly language?: string | null;
  readonly archive?: Text | null;
  readonly archiveLocation?: Text | null;
  readonly callNumber?: Text | null;
  readonly note?: Text | null;
  readonly abstractText?: Text | null;
  readonly genre?: Text | null;
  readonly keywords?: readonly string[];
  readonly parents?: readonly Entry[];
  readonly extensions?: Readonly<
    Record<string, Readonly<Record<string, ExtensionValue>>>
  >;
}

export interface TextChunk {
  readonly kind: "normal" | "protected" | "math";
  readonly text: string;
}
export interface Text {
  readonly chunks: readonly TextChunk[];
  readonly short?: readonly TextChunk[] | null;
}
export type Name =
  | {
      readonly kind: "person";
      readonly family: string;
      readonly given?: string | null;
      readonly prefix?: string | null;
      readonly suffix?: string | null;
      readonly alias?: string | null;
      readonly commaSuffix?: boolean;
      readonly id?: string | null;
      readonly givenInitials?: string | null;
      readonly prefixInitials?: string | null;
      readonly usePrefix?: boolean | null;
      readonly nonDroppingParticle?: string | null;
    }
  | { readonly kind: "organization"; readonly name: string };
export interface Contributors {
  readonly role: string;
  readonly names: readonly Name[];
}
export interface DateParts {
  readonly year: number;
  readonly month?: number | null;
  readonly day?: number | null;
  readonly season?: number | null;
  readonly time?: string | null;
}
export type DateValue =
  | { readonly kind: "point"; readonly date: DateParts }
  | {
      readonly kind: "range";
      readonly start?: DateParts | null;
      readonly end?: DateParts | null;
    }
  | { readonly kind: "literal"; readonly text: string };
export interface BibliographyDate {
  readonly value: DateValue;
  readonly uncertain?: boolean;
  readonly approximate?: boolean;
}
export interface ScalarValue {
  readonly kind: "typed" | "literal";
  readonly value: string;
}
export interface Publisher {
  readonly name?: Text | null;
  readonly location?: Text | null;
}
export interface BibliographyUrl {
  readonly value: string;
  readonly accessed?: BibliographyDate | null;
}
export type ExtensionValue =
  | null
  | boolean
  | number
  | string
  | readonly ExtensionValue[]
  | { readonly [key: string]: ExtensionValue };

export type ProjectionField =
  "key" | "entryType" | "title" | "date" | "doi" | "volume";
export interface ProjectionRow {
  key?: string;
  entryType?: string;
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
