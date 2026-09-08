import type { Diagnostic } from "./types.js";

export class RefkitError extends Error {
  override readonly name: string = "RefkitError";
}
export class ParseError extends RefkitError {
  override readonly name: string = "ParseError";
  readonly diagnostics: readonly Diagnostic[];
  constructor(message: string, diagnostics: readonly Diagnostic[] = []) {
    super(message);
    this.diagnostics = diagnostics;
  }
}
export class MissingReferenceError extends RefkitError {
  override readonly name: string = "MissingReferenceError";
}
export class TidyError extends RefkitError {
  override readonly name: string = "TidyError";
}
export class TidySyntaxError extends TidyError {
  override readonly name: string = "TidySyntaxError";
  readonly line: number;
  readonly column: number;
  readonly byte: number;
  readonly character: string | null;
  constructor(
    message: string,
    syntax: {
      line: number;
      column: number;
      byte: number;
      character: string | null;
    },
  ) {
    super(message);
    this.line = syntax.line;
    this.column = syntax.column;
    this.byte = syntax.byte;
    this.character = syntax.character;
  }
}

interface NativeError {
  name: string;
  message: string;
  diagnostics?: Diagnostic[];
  syntax?: {
    line: number;
    column: number;
    byte: number;
    character: string | null;
  };
}

function convertError(error: unknown): Error {
  if (error instanceof Error) return error;
  let value: NativeError;
  try {
    value = JSON.parse(String(error)) as NativeError;
    if (typeof value?.message !== "string")
      return new RefkitError(String(error));
  } catch {
    return new RefkitError(String(error));
  }
  switch (value.name) {
    case "ParseError":
      return new ParseError(value.message, value.diagnostics);
    case "MissingReferenceError":
      return new MissingReferenceError(value.message);
    case "TidySyntaxError":
      if (value.syntax) return new TidySyntaxError(value.message, value.syntax);
      return new TidyError(value.message);
    case "TidyError":
      return new TidyError(value.message);
    case "TypeError":
      return new TypeError(value.message);
    case "RangeError":
      return new RangeError(value.message);
    default:
      return new RefkitError(value.message);
  }
}

export function callNative<T>(operation: () => T): T {
  try {
    return operation();
  } catch (error) {
    throw convertError(error);
  }
}

export function readNative<T>(operation: () => string): T {
  return JSON.parse(callNative(operation)) as T;
}
