/** 32 bytes, supplied as 64 hex digits (optional 0x/separators) or a Uint8Array. */
export type Dna = string | Uint8Array;
/** Use DNA-selected timbres, or replace the melody with a synthesized piano. */
export type Timbre = "spec" | "piano";
/** Deterministic note algorithm. Version 1 reproduces legacy recordings. */
export type MusicVersion = 1 | 2;
/** Named ensemble parts. Each DNA's role byte must match its assigned part. */
export interface Participants {
  melody?: Dna;
  bass?: Dna;
  pad?: Dna;
  drums?: Dna;
}
/** Rendering parameters. Unknown properties and invalid values are rejected. */
export interface RenderOptions {
  /** Conductor DNA. Its own part cannot be replaced by a participant. */
  dna: Dna;
  /** Defaults to 2. Save this version with DNA to reproduce the same music. */
  musicVersion?: MusicVersion;
  /** Defaults to "spec". */
  timbre?: Timbre;
  /** Integer Hz, 22050–96000. Defaults to 44100 for WAV or context.sampleRate for AudioBuffer. */
  sampleRate?: number;
  /** Wrap tails into an exact 16-bar loop. Defaults to false. */
  loop?: boolean;
  /** Reverb amount, 0–1. Defaults to 0.18. */
  reverb?: number;
  /** Missing parts use the conductor's DNA. */
  participants?: Participants;
}
/** A self-contained PCM16 stereo WAV and its rendering metadata. */
export interface RenderedWav {
  /** Owned bytes, independent of wasm memory. Suitable for Blob or file writes. */
  readonly data: Uint8Array<ArrayBuffer>;
  /** Canonical, lowercase 64-digit conductor DNA. Save all participant DNA as well for ensembles. */
  readonly dna: string;
  readonly musicVersion: MusicVersion;
  readonly sampleRate: number;
  readonly channels: 2;
  /** Includes the release tail unless loop is true. */
  readonly durationSeconds: number;
}
/** The public library surface; no Rust handles, pointers, or manual free() calls. */
export interface Seedsong {
  /** Decode standard DOB/0 traits in Rust. */
  decodeDob0(options: Dob0Options): DobOutput[];
  /** Append standard DOB/1 SVG outputs in Rust; fragments are not sanitized. */
  decodeDob1(options: Dob1Options): DobOutput[];
  /** Generate music events and synthesis selectors, without rendering audio. */
  generateMusic(options: GenerateOptions): GeneratedMusic;
  /** Generate a cryptographically random DNA; save the returned string to reproduce it. */
  randomDna(): string;
  /** Synchronously render a complete WAV. Throws Error for invalid input or rendering failures. */
  renderWav(options: RenderOptions): RenderedWav;
  /**
   * Synchronously render stereo audio using a caller-owned AudioContext.
   * Does not resume, close, or otherwise manage the context or playback.
   * Render with loop: true when the buffer will be looped by AudioBufferSourceNode.
   */
  renderAudioBuffer(context: AudioContext, options: RenderOptions): AudioBuffer;
}

/** JSON values accepted by standard DOB patterns. Numbers must be JS-safe. */
export type JsonValue = null | boolean | number | string | JsonValue[] | { [key: string]: JsonValue };
/** The standard array representation is retained, including repeated trait names. */
export interface DobOutput {
  name: string;
  traits: Record<string, JsonValue>[];
}
export type Dob0Entry =
  | [name: string, type: string, offset: number, length: number, pattern: "options" | "range", args: JsonValue[]]
  | [name: string, type: string, offset: number, length: number, pattern: "rawNumber" | "rawString" | "utf8", args?: JsonValue[]];
export type Dob0Pattern = Dob0Entry[];
export type Dob1Condition = string | number | [number, number] | ["*"];
export type Dob1Entry =
  | [image: string, field: "attributes" | "elements", trait: string, pattern: "raw", template: string]
  | [image: string, field: "attributes" | "elements", trait: string, pattern: "options" | "range", branches: [Dob1Condition, string][]];
export type Dob1Pattern = Dob1Entry[];
export interface Dob0Options {
  /** Hex or bytes of any nonzero length; music still requires exactly 32 bytes. */
  dna: Dna;
  pattern: Dob0Pattern;
}
export interface Dob1Options {
  /** Standard DOB/0 output, not a flattened trait dictionary. */
  traits: DobOutput[];
  pattern: Dob1Pattern;
}
export type GenerateOptions = Pick<RenderOptions, "dna" | "musicVersion" | "participants">;
/** Start and duration are sixteenth-note steps; seconds per step = 60 / bpm / 4. */
export interface MusicEvent {
  start: number;
  duration: number;
  /** 0 melody, 1 bass, 2 pad, 3 drums. Drum pitches are spec-defined selectors. */
  part: number;
  pitch: number;
  velocity: number;
}
/** Integer synthesis/arrangement selectors; definitions and tables are in spec.md. */
export interface MusicParams {
  bpm: number;
  pc: number;
  mode: number;
  form: number;
  lead: number;
  bass: number;
  padPattern: number;
  bassPattern: number;
  drums: number;
  swing: number;
  role: number;
  tonic: number;
}
export interface GeneratedMusic {
  dna: string;
  musicVersion: MusicVersion;
  params: MusicParams;
  events: MusicEvent[];
  /** Canonical DNA in melody, bass, pad, drums order; includes ensemble sources. */
  sources: string[];
  /** Owned canonical SNG1 bytes; stable across native and WASM. */
  sng1: Uint8Array<ArrayBuffer>;
}
