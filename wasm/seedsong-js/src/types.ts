/** 32 bytes, supplied as 64 hex digits (optional 0x/separators) or a Uint8Array. */
export type Dna = string | Uint8Array;
/** Use DNA-selected timbres, or replace the melody with a synthesized piano. */
export type Timbre = "spec" | "piano";
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
  readonly sampleRate: number;
  readonly channels: 2;
  /** Includes the release tail unless loop is true. */
  readonly durationSeconds: number;
}
/** The public library surface; no Rust handles, pointers, or manual free() calls. */
export interface Seedsong {
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
