import * as crypto from 'crypto';

export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };

/**
 * Deterministically serialize JSON evidence.
 *
 * Object keys are sorted recursively. Array order is preserved because it is
 * semantically significant for event and observation sequences. Unsupported
 * JavaScript values and cycles are refused rather than silently discarded.
 */
export function canonicalJson(value: unknown): string {
  const ancestors = new Set<object>();

  const normalize = (candidate: unknown, path: string): JsonValue => {
    if (candidate === null || typeof candidate === 'string' || typeof candidate === 'boolean') {
      return candidate;
    }
    if (typeof candidate === 'number') {
      if (!Number.isFinite(candidate)) {
        throw new TypeError(`non-finite number at ${path}`);
      }
      return candidate;
    }
    if (Array.isArray(candidate)) {
      if (ancestors.has(candidate)) {
        throw new TypeError(`cyclic evidence at ${path}`);
      }
      ancestors.add(candidate);
      const result = candidate.map((item, index) => normalize(item, `${path}[${index}]`));
      ancestors.delete(candidate);
      return result;
    }
    if (typeof candidate === 'object') {
      const object = candidate as Record<string, unknown>;
      if (ancestors.has(object)) {
        throw new TypeError(`cyclic evidence at ${path}`);
      }
      ancestors.add(object);
      const result: Record<string, JsonValue> = {};
      for (const key of Object.keys(object).sort()) {
        const member = object[key];
        if (member === undefined || typeof member === 'function' || typeof member === 'symbol' || typeof member === 'bigint') {
          throw new TypeError(`unsupported evidence value at ${path}.${key}`);
        }
        result[key] = normalize(member, `${path}.${key}`);
      }
      ancestors.delete(object);
      return result;
    }
    throw new TypeError(`unsupported evidence value at ${path}`);
  };

  return JSON.stringify(normalize(value, '$'));
}

export function sha256Digest(value: unknown): string {
  return crypto.createHash('sha256').update(canonicalJson(value), 'utf8').digest('hex');
}

export function sha256Text(value: string): string {
  return crypto.createHash('sha256').update(value, 'utf8').digest('hex');
}
