import source from "../pkg/seedsong_core.js?source";
import { createLibrary } from "./api.js";
import type { Bindings } from "./api.js";
import type { Seedsong } from "./types.js";
export type * from "./types.js";

/** Raw WASM Cell data or an already compiled module; URLs are intentionally excluded. */
export type DecoderInput = Uint8Array<ArrayBuffer> | ArrayBuffer | (WebAssembly.Module & object);
/**
 * Instantiate a decoder fetched by the caller (for example through light-client-wasm).
 * No network calls or embedded WASM. Each call creates isolated bindings and memory.
 * Use the WASM distributed with this exact package build: wasm-bindgen imports are
 * build-specific. A compatible instance exposes decoder ABI version 1.
 */
export async function loadSeedsong(input: DecoderInput): Promise<Seedsong> {
  if (!(
    input instanceof Uint8Array ||
    input instanceof ArrayBuffer ||
    input instanceof WebAssembly.Module
  )) {
    throw new TypeError("Expected WASM bytes or a WebAssembly.Module");
  }
  // Compile before importing glue: malformed/incompatible Cells fail early.
  const module = input instanceof WebAssembly.Module ? input : await WebAssembly.compile(input);
  const exports = new Set(WebAssembly.Module.exports(module).map((value) => value.name));
  for (const name of [
    "decoderAbiVersion",
    "decodeDob0",
    "decodeDob1",
    "generateMusic",
    "renderWav",
    "renderAudioBuffer",
    "randomDna",
  ]) {
    if (!exports.has(name)) throw new Error(`Incompatible Seedsong decoder: missing ${name}`);
  }
  // wasm-bindgen's generated ESM has module-local mutable state. A distinct URL
  // gives each decoder its own bindings rather than sharing the first instance.
  // Node cannot import blob: URLs; its data: URL includes a unique fragment.
  const node =
    typeof globalThis === "object" &&
    "process" in globalThis &&
    !!(globalThis as { process?: { versions?: { node?: string } } }).process?.versions?.node;
  const url = node
    ? `data:text/javascript;base64,${btoa(Array.from(new TextEncoder().encode(source), (byte) => String.fromCharCode(byte)).join(""))}#seedsong-${globalThis.crypto.randomUUID()}`
    : URL.createObjectURL(new Blob([source], { type: "text/javascript" }));
  try {
    const bindings = (await import(/* webpackIgnore: true */ /* @vite-ignore */ url)) as Bindings;
    bindings.initSync({ module });
    if (bindings.decoderAbiVersion() !== 1)
      throw new Error("Unsupported Seedsong decoder ABI version");
    return createLibrary(bindings);
  } finally {
    if (!node) URL.revokeObjectURL(url);
  }
}
