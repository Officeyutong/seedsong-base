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
