export function string(value: unknown, name: string): string {
  if (typeof value !== "string")
    throw new TypeError(`${name} must be a string`);
  return value;
}

export function optionalString(value: unknown, name: string): string | null {
  return value == null ? null : string(value, name);
}

export function iterable<T>(value: Iterable<T>, name: string): T[] {
  if (
    typeof value === "string" ||
    value == null ||
    typeof value[Symbol.iterator] !== "function"
  ) {
    throw new TypeError(`${name} must be an iterable of items`);
  }
  return Array.from(value);
}

export function strings(value: Iterable<string>, name: string): string[] {
  return iterable(value, name).map((item) => string(item, name));
}

export function object(
  value: unknown,
  name: string,
  allowed?: readonly string[],
): void {
  if (
    value == null ||
    typeof value !== "object" ||
    Array.isArray(value) ||
    ![Object.prototype, null].includes(Object.getPrototypeOf(value))
  ) {
    throw new TypeError(`${name} must be an options object`);
  }
  if (allowed) {
    for (const key of Reflect.ownKeys(value)) {
      if (typeof key !== "string" || !allowed.includes(key)) {
        throw new TypeError(`unknown ${name} field ${String(key)}`);
      }
    }
  }
}
