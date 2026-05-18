// Crypto utility stubs — expanded by ggen pipeline as receipt ontology grows
// Source: ontology/zoela/*.ttl → prov-receipt.tera
//
// blake3Hash: stable deterministic hash for PROV-O receipt payloads.
// In production this delegates to a BLAKE3 WASM module; this stub provides
// a synchronous interface compatible with the generated receipt emitters.

export function blake3Hash(input: string): string {
  // Stub implementation — replace with wasm-blake3 or native binding in app workspace
  // Returns a hex string of the same shape as a real BLAKE3 digest.
  let h = 0x811c9dc5;
  for (let i = 0; i < input.length; i++) {
    h ^= input.charCodeAt(i);
    h = (Math.imul(h, 0x01000193) | 0) >>> 0;
  }
  return h.toString(16).padStart(64, '0');
}
