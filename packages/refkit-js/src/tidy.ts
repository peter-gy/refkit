import { getNative } from "./runtime.js";
import { readNative } from "./errors.js";
import { object, string } from "./inputs.js";
import type { TidyOptions, TidyResult } from "./types.js";

export interface TidySettings {
  options?: TidyOptions | null;
}

export function tidyBibtex(
  source: string,
  settings: TidySettings = {},
): TidyResult {
  const { tidy_bibtex } = getNative();
  string(source, "source");
  object(settings, "settings", ["options"]);
  const options = settings.options ?? {};
  object(options, "options");
  const input = JSON.stringify(
    options,
    function (this: Record<string, unknown>, key: string, value: unknown) {
      if (value === undefined && Array.isArray(this)) {
        throw new TypeError("tidy option arrays must contain defined values");
      }
      const original = key === "" ? options : this[key];
      if (
        original !== null &&
        typeof original === "object" &&
        !Array.isArray(original)
      ) {
        object(original, "tidy option");
        if (value !== original)
          throw new TypeError("tidy options must contain plain values");
      }
      if (typeof value === "number" && !Number.isFinite(value))
        throw new TypeError("tidy options require finite numbers");
      if (["function", "symbol", "bigint"].includes(typeof value)) {
        throw new TypeError("tidy options must contain JSON values");
      }
      return value;
    },
  );
  return readNative(() => tidy_bibtex(source, input));
}
