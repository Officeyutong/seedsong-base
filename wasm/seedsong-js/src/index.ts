import * as wasm from "../pkg/seedsong_core.js";
import bytes from "../pkg/seedsong_core_bg.wasm";
import type { Dna, RenderOptions, RenderedWav, Seedsong } from "./types.js";
export type { Dna, MusicVersion, Participants, RenderOptions, RenderedWav, Seedsong, Timbre } from "./types.js";

// Webpack embeds the wasm bytes; initialize once before exposing the library.
wasm.initSync({ module: bytes });

function normalizeDna(value: Dna): string {
  if (value instanceof Uint8Array) {
    if (value.length !== 32) throw new TypeError("DNA must contain exactly 32 bytes");
    return Array.from(value, byte => byte.toString(16).padStart(2, "0")).join("");
  }
  if (typeof value !== "string") throw new TypeError("DNA must be a hex string or Uint8Array");
  const hex = value.trim().replace(/^0x/i, "").replace(/[\s_-]/g, "");
  if (!/^[a-f\d]{64}$/i.test(hex)) throw new TypeError("DNA must contain exactly 64 hexadecimal digits");
  return hex.toLowerCase();
}
function normalizeOptions(options: RenderOptions): object {
  if (!options || typeof options !== "object" || Array.isArray(options)) {
    throw new TypeError("Render options must be an object with a dna field");
  }
  for (const key of ["sampleRate", "reverb", "musicVersion"] as const) {
    if (options[key] !== undefined && typeof options[key] !== "number") {
      throw new TypeError(`${key} must be a number`);
    }
  }
  if (options.loop !== undefined && typeof options.loop !== "boolean") throw new TypeError("loop must be a boolean");
  // Retain unknown fields for Rust bridge validation; undefined means omitted.
  const normalized: Record<string, unknown> = {
    ...Object.fromEntries(Object.entries(options).filter(([, value]) => value !== undefined)),
    dna: normalizeDna(options.dna),
  };
  if (options.participants !== undefined) {
    if (!options.participants || typeof options.participants !== "object" || Array.isArray(options.participants)) {
      throw new TypeError("participants must be an object keyed by part name");
    }
    normalized.participants = Object.fromEntries(Object.entries(options.participants)
      .filter(([, dna]) => dna !== undefined).map(([role, dna]) => [role, normalizeDna(dna)]));
  }
  return normalized;
}
const library: Seedsong = Object.freeze({
  randomDna: (): string => wasm.randomDna(),
  renderWav: (options: RenderOptions): RenderedWav => wasm.renderWav(normalizeOptions(options)) as RenderedWav,
  renderAudioBuffer: (context: AudioContext, options: RenderOptions): AudioBuffer => {
    if (!context || typeof context.createBuffer !== "function") throw new TypeError("An AudioContext is required");
    return wasm.renderAudioBuffer(context, normalizeOptions(options));
  },
});
/**
 * Return the music library. Bundled wasm is initialized once when this module loads.
 * No external wasm URL, fetch, or manual initialization is required.
 * This creates no AudioContext and requires no autoplay permission.
 * @example
 * const music = await createSeedsong();
 * const wav = music.renderWav({ dna: music.randomDna(), timbre: "piano" });
 * const blob = new Blob([wav.data], { type: "audio/wav" });
 */
export async function createSeedsong(): Promise<Seedsong> {
  return library;
}
