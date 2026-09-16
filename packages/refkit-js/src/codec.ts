import { getNative } from "./runtime.js";
import { callNative, readNative } from "./errors.js";
import { object, string } from "./inputs.js";
import { Library, libraryFromNative, nativeLibrary } from "./library.js";
import type {
  BibliographyFormat,
  ConversionIssue,
  ConversionReport,
  DecodeReport,
  EncodeReport,
  LossPolicy,
  RecoveryPolicy,
} from "./types.js";

export interface DecodeOptions {
  format: BibliographyFormat;
  loss?: LossPolicy;
  recovery?: RecoveryPolicy;
}
export interface EncodeOptions {
  format: BibliographyFormat;
  loss?: LossPolicy;
}
export interface ConvertOptions {
  sourceFormat: BibliographyFormat;
  targetFormat: BibliographyFormat;
  loss?: LossPolicy;
  recovery?: RecoveryPolicy;
}

export function decode(source: string, options: DecodeOptions): DecodeReport {
  object(options, "options", ["format", "loss", "recovery"]);
  const native = callNative(() =>
    getNative().decode_format(
      string(source, "source"),
      string(options.format, "format"),
      string(options.loss === undefined ? "report" : options.loss, "loss"),
      string(
        options.recovery === undefined ? "error" : options.recovery,
        "recovery",
      ),
    ),
  );
  return {
    library: libraryFromNative(callNative(() => native.library)),
    format: native.format as BibliographyFormat,
    issues: readNative<ConversionIssue[]>(() => native.issues()),
  };
}

export function encode(library: Library, options: EncodeOptions): EncodeReport {
  object(options, "options", ["format", "loss"]);
  return readNative(() =>
    getNative().encode_format(
      nativeLibrary(library),
      string(options.format, "format"),
      string(options.loss === undefined ? "report" : options.loss, "loss"),
    ),
  );
}

export function convert(
  source: string,
  options: ConvertOptions,
): ConversionReport {
  object(options, "options", [
    "sourceFormat",
    "targetFormat",
    "loss",
    "recovery",
  ]);
  return readNative(() =>
    getNative().convert_format(
      string(source, "source"),
      string(options.sourceFormat, "sourceFormat"),
      string(options.targetFormat, "targetFormat"),
      string(options.loss === undefined ? "report" : options.loss, "loss"),
      string(
        options.recovery === undefined ? "error" : options.recovery,
        "recovery",
      ),
    ),
  );
}
