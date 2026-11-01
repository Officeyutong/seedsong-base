import { createSeedsong } from "../dist/index.js";
import type { Dna, RenderOptions, RenderedWav } from "../dist/index.js";
const dna: Dna = new Uint8Array(32);
const options: RenderOptions = { dna, timbre: "piano", sampleRate: 48000, loop: true, participants: { bass: "00".repeat(32) } };
async function check(context: AudioContext) {
  const music = await createSeedsong();
  // @ts-expect-error Wasm bytes are bundled; initialization accepts no asset configuration.
  createSeedsong({ wasm: new Uint8Array(32) });
  const wav: RenderedWav = music.renderWav(options);
  const data: Uint8Array = wav.data;
  const channels: 2 = wav.channels;
  const musicVersion: 1 | 2 = wav.musicVersion;
  music.renderWav({ dna, musicVersion: 1 });
  // @ts-expect-error Only the two implemented algorithms are supported.
  music.renderWav({ dna, musicVersion: 3 });
  const buffer: AudioBuffer = music.renderAudioBuffer(context, { dna });
  // @ts-expect-error Named timbres replace ambiguous booleans.
  music.renderWav({ dna, timbre: true });
  // @ts-expect-error loop must be a boolean.
  music.renderWav({ dna, loop: "true" });
  // @ts-expect-error Arbitrary number arrays are not a DNA type.
  music.renderWav({ dna: [1, 2] });
  // @ts-expect-error Internal event encoders are not part of the API.
  music.encodeDna(dna);
  return { data, channels, musicVersion, buffer };
}
void check;

import { loadSeedsong } from "../dist/loader.js";
async function decoderTypes() {
  const library = await loadSeedsong(new Uint8Array());
  const traits = library.decodeDob0({ dna: "00", pattern: [["n", "Number", 0, 1, "rawNumber"]] });
  library.decodeDob1({ traits, pattern: [["IMAGE.0", "elements", "n", "range", [[[0, 255], "<g/>"]]]] });
  const events = library.generateMusic({ dna, musicVersion: 2 }).events;
  // @ts-expect-error Cell loaders accept bytes, never an implicit network URL.
  loadSeedsong("https://example.com/decoder.wasm");
  // @ts-expect-error Generation only accepts musical inputs, not render settings.
  library.generateMusic({ dna, sampleRate: 22050 });
  return events;
}
void decoderTypes;
