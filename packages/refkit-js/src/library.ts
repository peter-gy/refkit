import { NativeLibrary } from "./wasm/refkit_js_native.js";
import { assertInitialized } from "./runtime.js";
import { callNative, readNative } from "./errors.js";
import { object, string, strings } from "./inputs.js";
import type {
  Diagnostic,
  Entry,
  ProjectionField,
  ProjectionRow,
  RecoveryPolicy,
} from "./types.js";

export interface ParseOptions {
  recovery?: RecoveryPolicy;
}
export interface ProjectOptions {
  keys?: Iterable<string> | null;
}
const libraries = new WeakMap<Library, NativeLibrary>();
const sourceDiagnostics = new WeakMap<Library, readonly Diagnostic[]>();

export function nativeLibrary(library: Library): NativeLibrary {
  const native = libraries.get(library);
  if (!native) throw new TypeError("library must be a Library");
  return native;
}

export function setLibraryDecodingDiagnostic(
  library: Library,
  diagnostic: Diagnostic | null,
): void {
  sourceDiagnostics.set(library, diagnostic ? [diagnostic] : []);
}

export class Library implements Iterable<Entry> {
  private constructor(native: NativeLibrary) {
    libraries.set(this, native);
  }

  static parseBibtex(source: string, options: ParseOptions = {}): Library {
    assertInitialized();
    object(options, "options", ["recovery"]);
    return new Library(
      callNative(() =>
        NativeLibrary.parse_bibtex(
          string(source, "source"),
          string(
            options.recovery === undefined ? "error" : options.recovery,
            "recovery",
          ),
        ),
      ),
    );
  }

  static parseYaml(source: string): Library {
    assertInitialized();
    return new Library(
      callNative(() => NativeLibrary.parse_yaml(string(source, "source"))),
    );
  }

  get diagnostics(): readonly Diagnostic[] {
    return [
      ...structuredClone(sourceDiagnostics.get(this) ?? []),
      ...readNative<Diagnostic[]>(() => nativeLibrary(this).diagnostics()),
    ];
  }
  get size(): number {
    return callNative(() => nativeLibrary(this).size);
  }
  keys(): string[] {
    return readNative(() => nativeLibrary(this).keys());
  }
  values(): Entry[] {
    return readNative(() => nativeLibrary(this).records());
  }
  get(key: string): Entry | null {
    return readNative(() => nativeLibrary(this).get_record(string(key, "key")));
  }
  getMany(keys: Iterable<string>): Entry[] {
    const input = JSON.stringify(strings(keys, "keys"));
    return readNative(() => nativeLibrary(this).get_many(input));
  }
  has(key: string): boolean {
    return this.get(key) !== null;
  }
  isEmpty(): boolean {
    return this.size === 0;
  }
  select(selector: string): Entry[] {
    return readNative(() =>
      nativeLibrary(this).select_records(string(selector, "selector")),
    );
  }
  project(
    fields?: Iterable<ProjectionField> | null,
    options: ProjectOptions = {},
  ): ProjectionRow[] {
    object(options, "options", ["keys"]);
    const fieldsJson = JSON.stringify(
      fields == null ? null : strings(fields, "fields"),
    );
    const keysJson = JSON.stringify(
      options.keys == null ? null : strings(options.keys, "keys"),
    );
    return readNative(() => nativeLibrary(this).project(fieldsJson, keysJson));
  }
  [Symbol.iterator](): Iterator<Entry> {
    return this.values()[Symbol.iterator]();
  }
}
