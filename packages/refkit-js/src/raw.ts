import type { NativeRawDocument } from "./wasm/refkit_js_native.js";
import { getNative } from "./runtime.js";
import { callNative, readNative } from "./errors.js";
import {
  string,
  iterable,
  jsonData,
  object,
  optionalString,
} from "./inputs.js";
import { tidyBibtex, type TidySettings } from "./tidy.js";
import type {
  Diagnostic,
  RawBlock,
  RawFailedBlock,
  RawSpan,
  ResolvedBibEntry,
  TidyResult,
  ValidationReport,
  BibEdit,
  BibPatchResult,
  RawEntryInfo as EntryInfo,
  RawFieldInfo as FieldInfo,
  DuplicateRule,
  DuplicateReport,
  MergeFieldChoice,
  MergePlan,
} from "./types.js";

export interface DuplicateOptions {
  rules?: Iterable<DuplicateRule> | null;
}
export interface MergeOptions {
  entries: Iterable<number>;
  retain: number;
  fields?: Iterable<MergeFieldChoice> | null;
  entryType?: string | null;
}

let entryMap: (native: NativeRawDocument) => BibEntryMap;
let entry: (native: NativeRawDocument, info: EntryInfo) => BibEntry;
let fieldMap: (native: NativeRawDocument, entryId: number) => BibFieldMap;
let field: (entryId: number, info: FieldInfo) => BibField;
const sourceDiagnostics = new WeakMap<BibDocument, readonly Diagnostic[]>();

export function setBibDecodingDiagnostic(
  document: BibDocument,
  diagnostic: Diagnostic | null,
): void {
  sourceDiagnostics.set(document, diagnostic ? [diagnostic] : []);
}

export class BibDocument {
  readonly #native: NativeRawDocument;
  private constructor(native: NativeRawDocument) {
    this.#native = native;
  }
  static parse(source: string): BibDocument {
    const { NativeRawDocument } = getNative();
    return new BibDocument(
      callNative(() => NativeRawDocument.parse(string(source, "source"))),
    );
  }
  applyPatch(patch: Iterable<BibEdit>): BibPatchResult {
    const input = jsonData(iterable(patch, "patch"), "patch");
    const result = callNative(() => this.#native.apply_patch(input));
    const report = readNative<Omit<BibPatchResult, "document">>(() =>
      result.report(),
    );
    return {
      document: new BibDocument(callNative(() => result.document)),
      ...report,
    };
  }
  findDuplicates(options: DuplicateOptions = {}): DuplicateReport {
    object(options, "options", ["rules"]);
    const rules =
      options.rules == null ? null : iterable(options.rules, "rules");
    return readNative(() =>
      this.#native.find_duplicates(jsonData(rules, "rules")),
    );
  }
  planMerge(options: MergeOptions): MergePlan {
    object(options, "options", ["entries", "retain", "fields", "entryType"]);
    const request = jsonData(
      {
        entries: iterable(options.entries, "entries"),
        retain: options.retain,
        fields:
          options.fields == null ? [] : iterable(options.fields, "fields"),
        entryType: optionalString(options.entryType, "entryType"),
      },
      "merge request",
    );
    return readNative(() => this.#native.plan_merge(request));
  }
  get entries(): BibEntryMap {
    return entryMap(this.#native);
  }
  get diagnostics(): readonly Diagnostic[] {
    return structuredClone(sourceDiagnostics.get(this) ?? []);
  }
  get comments(): string[] {
    return readNative(() => this.#native.comments());
  }
  get preamble(): string {
    return callNative(() => this.#native.preamble());
  }
  get strings(): Record<string, string> {
    return readNative(() => this.#native.strings());
  }
  get failedBlocks(): RawFailedBlock[] {
    return readNative(() => this.#native.failed_blocks());
  }
  get blocks(): RawBlock[] {
    return readNative(() => this.#native.blocks());
  }
  toBibtex(): string {
    return callNative(() => this.#native.to_bibtex());
  }
  resolve(): readonly ResolvedBibEntry[] {
    return readNative(() => this.#native.resolve());
  }
  validate(): ValidationReport {
    return readNative(() => this.#native.validate());
  }
  tidy(settings: TidySettings = {}): TidyResult {
    return tidyBibtex(this.toBibtex(), settings);
  }
}

export class BibEntryMap implements Iterable<BibEntry> {
  readonly #native: NativeRawDocument;
  private constructor(native: NativeRawDocument) {
    this.#native = native;
  }
  static {
    entryMap = (native) => new BibEntryMap(native);
  }
  #records(): EntryInfo[] {
    return readNative(() => this.#native.entries());
  }
  uniqueKeys(): string[] {
    return readNative(() => this.#native.entry_keys());
  }
  occurrenceKeys(): string[] {
    return this.#records().map((info) => info.key);
  }
  occurrences(): BibEntry[] {
    return this.#records().map((info) => entry(this.#native, info));
  }
  getAll(key: string): BibEntry[] {
    const records = readNative<EntryInfo[]>(() =>
      this.#native.entries_for_key(string(key, "key")),
    );
    return records.map((info) => entry(this.#native, info));
  }
  getUnique(key: string): BibEntry | null {
    const info = readNative<EntryInfo | null>(() =>
      this.#native.unique_entry(string(key, "key")),
    );
    return info === null ? null : entry(this.#native, info);
  }
  get size(): number {
    return callNative(() => this.#native.entry_count());
  }
  isEmpty(): boolean {
    return this.size === 0;
  }
  has(key: string): boolean {
    return callNative(() => this.#native.contains_entry(string(key, "key")));
  }
  [Symbol.iterator](): Iterator<BibEntry> {
    return this.occurrences()[Symbol.iterator]();
  }
}

export class BibEntry {
  readonly #native: NativeRawDocument;
  readonly #info: EntryInfo;
  private constructor(native: NativeRawDocument, info: EntryInfo) {
    this.#native = native;
    this.#info = info;
  }
  static {
    entry = (native, info) => new BibEntry(native, info);
  }
  get key(): string {
    return this.#info.key;
  }
  get id(): number {
    return this.#info.id;
  }
  get kind(): string {
    return this.#info.kind;
  }
  get span(): RawSpan {
    return [...this.#info.span];
  }
  get fields(): BibFieldMap {
    return fieldMap(this.#native, this.#info.id);
  }
}

export class BibFieldMap implements Iterable<BibField> {
  readonly #native: NativeRawDocument;
  readonly #entryId: number;
  private constructor(native: NativeRawDocument, entryId: number) {
    this.#native = native;
    this.#entryId = entryId;
  }
  static {
    fieldMap = (native, entryId) => new BibFieldMap(native, entryId);
  }
  #records(): FieldInfo[] {
    return readNative(() => this.#native.fields(this.#entryId));
  }
  uniqueKeys(): string[] {
    return readNative(() => this.#native.field_keys(this.#entryId));
  }
  occurrenceKeys(): string[] {
    return this.#records().map((info) => info.name);
  }
  occurrences(): BibField[] {
    return this.#records().map((info) => field(this.#entryId, info));
  }
  getAll(key: string): BibField[] {
    const records = readNative<FieldInfo[]>(() =>
      this.#native.fields_for_key(this.#entryId, string(key, "key")),
    );
    return records.map((info) => field(this.#entryId, info));
  }
  getUnique(key: string): BibField | null {
    const info = readNative<FieldInfo | null>(() =>
      this.#native.unique_field(this.#entryId, string(key, "key")),
    );
    return info === null ? null : field(this.#entryId, info);
  }
  get size(): number {
    return callNative(() => this.#native.field_count(this.#entryId));
  }
  isEmpty(): boolean {
    return this.size === 0;
  }
  has(key: string): boolean {
    return callNative(() =>
      this.#native.contains_field(this.#entryId, string(key, "key")),
    );
  }
  [Symbol.iterator](): Iterator<BibField> {
    return this.occurrences()[Symbol.iterator]();
  }
}

export class BibField {
  readonly #entryId: number;
  readonly #info: FieldInfo;
  private constructor(entryId: number, info: FieldInfo) {
    this.#entryId = entryId;
    this.#info = info;
  }
  static {
    field = (entryId, info) => new BibField(entryId, info);
  }
  get name(): string {
    return this.#info.name;
  }
  get span(): RawSpan {
    return [...this.#info.span];
  }
  get value(): string {
    return this.#info.value;
  }
  get id(): number {
    return this.#info.id;
  }
  get entryId(): number {
    return this.#entryId;
  }
}
