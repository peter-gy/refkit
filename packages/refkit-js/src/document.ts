import {
  NativeDocument,
  NativeStyle,
  is_bundled_locale,
} from "./wasm/refkit_js_native.js";
import { assertInitialized } from "./runtime.js";
import { callNative, readNative, MissingReferenceError } from "./errors.js";
import { object, string } from "./inputs.js";
import { Library, nativeLibrary } from "./library.js";
import { Citation, citationsJson, type CitationInput } from "./citation.js";
import type { Rendered } from "./types.js";

const styles = new WeakMap<Style, NativeStyle>();
const styleSources = new WeakMap<Style, string>();
export function setStyleSource(style: Style, source: string): void {
  styleSources.set(style, source);
}
function nativeStyle(style: Style): NativeStyle {
  const native = styles.get(style);
  if (!native) throw new TypeError("style must be a Style");
  return native;
}

export class Style {
  private constructor(native: NativeStyle) {
    styles.set(this, native);
  }
  static load(name: string): Style {
    assertInitialized();
    return new Style(callNative(() => NativeStyle.load(string(name, "name"))));
  }
  static fromXml(xml: string): Style {
    assertInitialized();
    return new Style(
      callNative(() => NativeStyle.from_xml(string(xml, "xml"))),
    );
  }
  get id(): string {
    const id = callNative(() => nativeStyle(this).id);
    return styleSources.get(this) ?? id;
  }
  get title(): string {
    return callNative(() => nativeStyle(this).title);
  }
}

export class Locale {
  private constructor(readonly code: string) {
    Object.freeze(this);
  }
  static load(code: string): Locale {
    assertInitialized();
    string(code, "code");
    if (!callNative(() => is_bundled_locale(code)))
      throw new RangeError(`unknown bundled locale ${JSON.stringify(code)}`);
    return new Locale(code);
  }
}

export interface DocumentOptions {
  locale?: string | Locale | null;
}
export interface RenderOptions extends DocumentOptions {
  style?: string | Style;
}
interface RenderedDocumentData {
  citationOrder: string[];
  citations: Record<string, Rendered>;
  bibliography: Rendered;
}
let renderedDocument: (data: RenderedDocumentData) => RenderedDocument;

export class RenderedDocument {
  readonly citationOrder: readonly string[];
  readonly citations: Readonly<Record<string, Rendered>>;
  readonly bibliography: Rendered;
  private constructor(data: RenderedDocumentData) {
    this.citationOrder = Object.freeze(data.citationOrder);
    this.citations = Object.freeze(data.citations);
    this.bibliography = data.bibliography;
    Object.freeze(this);
  }
  static {
    renderedDocument = (data) => new RenderedDocument(data);
  }
  get(id: string): Rendered {
    string(id, "id");
    if (!Object.hasOwn(this.citations, id))
      throw new MissingReferenceError(
        `unknown citation id ${JSON.stringify(id)}`,
      );
    return this.citations[id]!;
  }
}

export class Document {
  readonly #native: NativeDocument;
  constructor(library: Library, style: Style, options: DocumentOptions = {}) {
    assertInitialized();
    object(options, "options", ["locale"]);
    const locale =
      options.locale instanceof Locale ? options.locale.code : options.locale;
    if (locale != null) string(locale, "locale");
    this.#native = callNative(
      () =>
        new NativeDocument(
          nativeLibrary(library),
          nativeStyle(style),
          locale ?? undefined,
        ),
    );
  }
  render(citations: Iterable<Citation>): RenderedDocument {
    const input = citationsJson(citations);
    return renderedDocument(readNative(() => this.#native.render(input)));
  }
  citedBibliography(citations: Iterable<Citation>): Rendered {
    const input = citationsJson(citations);
    return readNative(() => this.#native.cited_bibliography(input));
  }
  fullBibliography(): Rendered {
    return readNative(() => this.#native.full_bibliography());
  }
}

function withDocument<T>(
  library: Library,
  options: RenderOptions,
  render: (document: Document) => T,
): T {
  object(options, "options", ["style", "locale"]);
  const style =
    options.style instanceof Style
      ? options.style
      : Style.load(options.style === undefined ? "apa" : options.style);
  const document = new Document(library, style, {
    locale: options.locale === undefined ? "en-US" : options.locale,
  });
  return render(document);
}

export function cite(
  library: Library,
  citation: CitationInput,
  options: RenderOptions = {},
): Rendered {
  return withDocument(library, options, (document) =>
    document.render([new Citation("citation", citation)]).get("citation"),
  );
}

export function fullBibliography(
  library: Library,
  options: RenderOptions = {},
): Rendered {
  return withDocument(library, options, (document) =>
    document.fullBibliography(),
  );
}
