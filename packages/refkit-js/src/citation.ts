import { iterable, object, optionalString, string } from "./inputs.js";

export interface CiteOptions {
  locator?: string | null;
  label?: string | null;
}
export interface CitationOptions {
  noteNumber?: number | null;
}
export type CitationInput = string | Cite | CitationGroup;

export class Cite {
  readonly key: string;
  readonly locator: string | null;
  readonly label: string | null;
  constructor(key: string, options: CiteOptions = {}) {
    object(options, "options", ["locator", "label"]);
    this.key = string(key, "key");
    this.locator = optionalString(options.locator, "locator");
    this.label = optionalString(options.label, "label");
    Object.freeze(this);
  }
}

function asCite(item: string | Cite): Cite {
  if (item instanceof Cite) return item;
  if (typeof item === "string") return new Cite(item);
  throw new TypeError("citation items must be key strings or Cite objects");
}

export class CitationGroup implements Iterable<Cite> {
  readonly items: readonly Cite[];
  constructor(items: Iterable<string | Cite>) {
    const cites = iterable(items, "items").map(asCite);
    if (cites.length === 0)
      throw new RangeError("CitationGroup requires at least one citation");
    this.items = Object.freeze(cites);
    Object.freeze(this);
  }
  get size(): number {
    return this.items.length;
  }
  [Symbol.iterator](): Iterator<Cite> {
    return this.items[Symbol.iterator]();
  }
}

export class Citation {
  readonly id: string;
  readonly group: CitationGroup;
  readonly noteNumber: number | null;
  constructor(
    id: string,
    citation: CitationInput,
    options: CitationOptions = {},
  ) {
    object(options, "options", ["noteNumber"]);
    this.id = string(id, "id");
    this.group =
      citation instanceof CitationGroup
        ? citation
        : new CitationGroup([asCite(citation)]);
    const noteNumber = options.noteNumber ?? null;
    if (
      noteNumber !== null &&
      (!Number.isInteger(noteNumber) ||
        noteNumber < 1 ||
        noteNumber > 4_294_967_295)
    ) {
      throw new RangeError(
        "noteNumber must be an integer from 1 to 4294967295",
      );
    }
    this.noteNumber = noteNumber;
    Object.freeze(this);
  }
}

export function citationsJson(citations: Iterable<Citation>): string {
  const values = iterable(citations, "citations").map((citation) => {
    if (!(citation instanceof Citation))
      throw new TypeError("citations must contain Citation objects");
    return {
      id: citation.id,
      items: citation.group.items,
      noteNumber: citation.noteNumber,
    };
  });
  return JSON.stringify(values);
}
