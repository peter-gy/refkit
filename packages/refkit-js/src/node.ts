import { readFileSync } from "node:fs";
import { readFile, writeFile } from "node:fs/promises";
import { extname } from "node:path";
import { fileURLToPath } from "node:url";
import { decode_bibliography } from "./wasm/refkit_js_native.js";
import { initializeSync } from "./runtime.js";
import { readNative, RefkitError } from "./errors.js";
import { object, string } from "./inputs.js";
import {
  Library,
  setLibraryDecodingDiagnostic,
  type ParseOptions,
} from "./library.js";
import { Style, setStyleSource } from "./document.js";
import { BibDocument, setBibDecodingDiagnostic } from "./raw.js";
import type { TidySettings } from "./tidy.js";
import type { Diagnostic, TidyResult } from "./types.js";

initializeSync(
  readFileSync(new URL("./wasm/refkit_js_native_bg.wasm", import.meta.url)),
);

export * from "./index.js";
export type FilePath = string | URL;
export interface TidyFileSettings extends TidySettings {
  output?: FilePath | null;
}

function filePath(path: FilePath): string {
  if (path instanceof URL) return fileURLToPath(path);
  if (typeof path !== "string")
    throw new TypeError("path must be a string or file URL");
  return path;
}

async function readBytes(path: FilePath): Promise<Uint8Array> {
  const name = filePath(path);
  try {
    return await readFile(name);
  } catch (cause) {
    throw new RefkitError(`failed to read ${name}: ${String(cause)}`, {
      cause,
    });
  }
}

async function readBibliography(
  path: FilePath,
): Promise<{ text: string; diagnostic: Diagnostic | null }> {
  const bytes = await readBytes(path);
  return readNative(() => decode_bibliography(bytes));
}

export async function readLibrary(
  path: FilePath,
  options: ParseOptions = {},
): Promise<Library> {
  object(options, "options", ["recovery"]);
  const recovery = string(
    options.recovery === undefined ? "error" : options.recovery,
    "recovery",
  );
  if (recovery !== "error" && recovery !== "report")
    throw new RangeError("recovery must be 'error' or 'report'");
  const extension = extname(filePath(path)).toLowerCase();
  if (![".bib", ".yaml", ".yml"].includes(extension)) {
    throw new RefkitError(
      extension
        ? `unsupported bibliography extension ${JSON.stringify(extension.slice(1))}`
        : "bibliography path has no extension",
    );
  }
  const source = await readBibliography(path);
  const library =
    extension === ".bib"
      ? Library.parseBibtex(source.text, { recovery })
      : Library.parseYaml(source.text);
  setLibraryDecodingDiagnostic(library, source.diagnostic);
  return library;
}

export async function readBibDocument(path: FilePath): Promise<BibDocument> {
  const source = await readBibliography(path);
  const document = BibDocument.parse(source.text);
  setBibDecodingDiagnostic(document, source.diagnostic);
  return document;
}

export async function readStyle(path: FilePath): Promise<Style> {
  const bytes = await readBytes(path);
  let xml: string;
  try {
    xml = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch (cause) {
    throw new RefkitError(`failed to read style ${filePath(path)} as UTF-8`, {
      cause,
    });
  }
  const style = Style.fromXml(xml);
  setStyleSource(style, filePath(path));
  return style;
}

export async function tidyFile(
  path: FilePath,
  settings: TidyFileSettings = {},
): Promise<TidyResult> {
  object(settings, "settings", ["options", "output"]);
  const output = settings.output == null ? null : filePath(settings.output);
  const document = await readBibDocument(path);
  const result = document.tidy({ options: settings.options ?? null });
  if (output !== null) {
    try {
      await writeFile(output, result.bibtex, "utf8");
    } catch (cause) {
      throw new RefkitError(
        `failed to write BibTeX ${output}: ${String(cause)}`,
        { cause },
      );
    }
  }
  return result;
}
