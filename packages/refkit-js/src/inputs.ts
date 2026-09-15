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

export function jsonData(value: unknown, name: string): string {
  const pending: unknown[] = [value];
  const seen = new WeakSet<object>();
  while (pending.length) {
    const item = pending.pop();
    if (item == null || typeof item === "string" || typeof item === "boolean")
      continue;
    if (typeof item === "number") {
      if (!Number.isFinite(item))
        throw new RangeError(`${name} numbers must be finite`);
      continue;
    }
    if (typeof item !== "object")
      throw new TypeError(`${name} must contain JSON data`);
    if (seen.has(item)) continue;
    seen.add(item);
    if (Array.isArray(item)) {
      for (const child of item) pending.push(child);
    } else {
      object(item, name);
      for (const key of Reflect.ownKeys(item)) {
        if (typeof key !== "string")
          throw new TypeError(`${name} keys must be strings`);
        const property = Object.getOwnPropertyDescriptor(item, key)!;
        if (!("value" in property))
          throw new TypeError(`${name} must contain data properties`);
        if (property.enumerable) pending.push(property.value);
      }
    }
  }
  const result = JSON.stringify(value);
  if (result === undefined)
    throw new TypeError(`${name} must contain JSON data`);
  return result;
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
