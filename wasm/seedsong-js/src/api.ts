import type {
  Dna,
  RenderOptions,
  RenderedWav,
  Seedsong,
  Dob0Options,
  Dob1Options,
  DobOutput,
  GenerateOptions,
  GeneratedMusic,
} from "./types.js";
export type Bindings = Pick<
  typeof import("../pkg/seedsong_core.js"),
  | "randomDna"
  | "renderWav"
  | "renderAudioBuffer"
  | "decodeDob0"
  | "decodeDob1"
  | "generateMusic"
  | "decoderAbiVersion"
  | "initSync"
>;
function normalizeDna(value: Dna, music = true): string {
  if (value instanceof Uint8Array) {
    if ((music && value.length !== 32) || !value.length)
      throw new TypeError(music ? "DNA must contain exactly 32 bytes" : "DOB DNA must not be empty");
    return Array.from(value, (byte) => byte.toString(16).padStart(2, "0")).join("");
  }
  if (typeof value !== "string") throw new TypeError("DNA must be a hex string or Uint8Array");
  const hex = value
    .trim()
    .replace(/^0x/i, "")
    .replace(/[\s_-]/g, "");
  if (!(music ? /^[a-f\d]{64}$/i : /^(?:[a-f\d]{2})+$/i).test(hex))
    throw new TypeError(
      music ? "DNA must contain exactly 64 hexadecimal digits" : "DOB DNA must contain hexadecimal byte pairs",
    );
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
  if (options.loop !== undefined && typeof options.loop !== "boolean")
    throw new TypeError("loop must be a boolean");
  // Retain unknown fields for Rust bridge validation; undefined means omitted.
  const normalized: Record<string, unknown> = {
    ...Object.fromEntries(Object.entries(options).filter(([, value]) => value !== undefined)),
    dna: normalizeDna(options.dna),
  };
  if (options.participants !== undefined) {
    if (
      !options.participants ||
      typeof options.participants !== "object" ||
      Array.isArray(options.participants)
    ) {
      throw new TypeError("participants must be an object keyed by part name");
    }
    normalized.participants = Object.fromEntries(
      Object.entries(options.participants)
        .filter(([, dna]) => dna !== undefined)
        .map(([role, dna]) => [role, normalizeDna(dna)]),
    );
  }
  return normalized;
}
export function createLibrary(wasm: Bindings): Seedsong {
  return Object.freeze({
    decodeDob0: (options: Dob0Options): DobOutput[] =>
      wasm.decodeDob0({ ...options, dna: normalizeDna(options.dna, false) }) as DobOutput[],
    decodeDob1: (options: Dob1Options): DobOutput[] => wasm.decodeDob1(options) as DobOutput[],
    generateMusic: (options: GenerateOptions): GeneratedMusic =>
      wasm.generateMusic(normalizeOptions(options)) as GeneratedMusic,
    randomDna: (): string => wasm.randomDna(),
    renderWav: (options: RenderOptions): RenderedWav =>
      wasm.renderWav(normalizeOptions(options)) as RenderedWav,
    renderAudioBuffer: (context: AudioContext, options: RenderOptions): AudioBuffer => {
      if (!context || typeof context.createBuffer !== "function")
        throw new TypeError("An AudioContext is required");
      return wasm.renderAudioBuffer(context, normalizeOptions(options));
    },
  });
}
