# seedsong-js

Generate music from 32-byte DNA in JavaScript or TypeScript. The library provides WAV files and WebAudio buffers through a small typed API. Its ESM bundle includes the Rust wasm program and has no runtime npm dependencies.

For the Rust CLI, specification, and repository-wide checks, see the [project README](../../README.md).

## Build and install

Building from source requires Rust with edition 2024 support, the wasm target, wasm-pack, Node.js 22+, and npm. From the repository root:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
npm install
npm run build:js
```

Use `npm run build:js:debug` for a development build. From this package directory, the equivalent commands are `npm run build` and `npm run build-debug`. The repository ignores `package-lock.json`; use `npm install` on a fresh checkout.

Install the built package into another application:

```bash
npm install /absolute/path/to/seedsong-base/wasm/seedsong-js
```

Consumers need an ESM environment supporting WebAssembly. Rendering an `AudioBuffer` requires a browser `AudioContext`; WAV generation also works in Node.js. The built library initializes its embedded wasm when imported and requires no asset URL, fetch configuration, or manual memory management.

## Quick start: WAV output

```typescript
import { createSeedsong } from 'seedsong-js';

const music = await createSeedsong();
const wav = music.renderWav({
  dna: music.randomDna(),
  timbre: 'piano',
  sampleRate: 44100,
  reverb: 0.18,
  loop: false,
});

const blob = new Blob([wav.data], { type: 'audio/wav' });
const url = URL.createObjectURL(blob);
```

Use the URL as an audio element's source or an anchor's `href` with a `download` attribute. Revoke it with `URL.revokeObjectURL(url)` after it is no longer needed. Start browser playback from a user gesture when required.

`wav.data` is an owned `Uint8Array` that remains valid after later renders and wasm memory growth. Save `wav.dna`, the render options, and any participant DNA to reproduce a result.

## Public API

The only runtime export is:

```typescript
createSeedsong(): Promise<Seedsong>
```

It takes no arguments and returns the shared library object. The wasm module is initialized once, synchronously during import, using `initSync`. Creating the library does not create an AudioContext or start playback.

| Method | Return type | Purpose |
| --- | --- | --- |
| `randomDna()` | `string` | Generate cryptographically random DNA as 64 lowercase hex digits |
| `renderWav(options)` | `RenderedWav` | Generate a complete stereo PCM16 WAV with metadata |
| `renderAudioBuffer(context, options)` | `AudioBuffer` | Generate stereo audio for a caller-owned `AudioContext` |

The package also exports TypeScript types: `Dna`, `Timbre`, `Participants`, `RenderOptions`, `RenderedWav`, and `Seedsong`.

### Render options

```typescript
interface RenderOptions {
  dna: string | Uint8Array;
  timbre?: 'spec' | 'piano';
  sampleRate?: number;
  loop?: boolean;
  reverb?: number;
  participants?: {
    melody?: string | Uint8Array;
    bass?: string | Uint8Array;
    pad?: string | Uint8Array;
    drums?: string | Uint8Array;
  };
}
```

| Field | Meaning | Default |
| --- | --- | --- |
| `dna` | Conductor DNA: 64 hex digits or exactly 32 bytes in a `Uint8Array` | Required |
| `timbre` | `"spec"` uses DNA-selected timbres; `"piano"` replaces the melody sound | `"spec"` |
| `sampleRate` | Integer Hz between 22050 and 96000 | 44100 for WAV; `context.sampleRate` for AudioBuffer |
| `loop` | Produce exactly 16 bars, wrapping release and reverb tails to the beginning | `false` |
| `reverb` | Finite number from 0 to 1 | `0.18` |
| `participants` | Optional DNA for named ensemble parts | Missing parts use conductor DNA |

Hex input accepts an optional `0x` prefix, whitespace, underscores, and hyphens, and is normalized to lowercase without separators. Plain number arrays are not accepted as DNA. Optional fields set to `undefined` are treated as omitted.

### WAV result

```typescript
interface RenderedWav {
  readonly data: Uint8Array<ArrayBuffer>;
  readonly dna: string;
  readonly sampleRate: number;
  readonly channels: 2;
  readonly durationSeconds: number;
}
```

`dna` is the normalized conductor DNA. Duration includes the natural release tail unless `loop` is true. The output always contains stereo PCM16 WAV bytes, suitable for a Blob or a file write.

## WebAudio playback

The caller owns the AudioContext, sources, and connections. Rendering returns a buffer without resuming or closing the context or starting playback.

```typescript
import { createSeedsong } from 'seedsong-js';

const music = await createSeedsong();
const context = new AudioContext();
const buffer = music.renderAudioBuffer(context, {
  dna: music.randomDna(),
  timbre: 'piano',
  loop: true,
});
let source: AudioBufferSourceNode | undefined;

// Call from a user click handler.
async function play() {
  stop();
  await context.resume();
  source = context.createBufferSource();
  source.buffer = buffer;
  source.loop = true;
  source.connect(context.destination);
  source.start();
}

function stop() {
  if (source) {
    source.stop();
    source.disconnect();
    source = undefined;
  }
}

async function dispose() {
  stop();
  await context.close();
}
```

Cache buffers for repeated playback. For looping, set both render option `loop: true` and the source's `loop` property. Browser autoplay rules apply when resuming the context or starting audible playback.

## Node.js file export

The same bundle runs in Node.js without loading a separate wasm file:

```javascript
import { writeFile } from 'node:fs/promises';
import { createSeedsong } from 'seedsong-js';

const music = await createSeedsong();
const wav = music.renderWav({ dna: music.randomDna(), timbre: 'piano' });
await writeFile('song.wav', wav.data);
```

## Ensembles

The conductor controls key, mode, tempo, form, harmony, and swing. Each participant supplies the parameters for its own part. Its role byte must match that part: `D[16] % 4` is melody 0, bass 1, pad/harmony 2, or drums/rhythm 3. The conductor performs its own part and cannot be replaced.

```typescript
import { createSeedsong } from 'seedsong-js';

const music = await createSeedsong();
const conductor = new Uint8Array(32); // Melody role.
const bass = conductor.slice();
bass[16] = 1;  // Bass role.
bass[18] = 12; // Select the octave-alternating bass pattern.

const wav = music.renderWav({
  dna: conductor,
  participants: { bass },
  timbre: 'piano',
});
```

Missing parts use the conductor's DNA. Record all participant DNA as well as the conductor DNA for reproducibility.

## Errors and execution model

Both render methods are synchronous and occupy the current JS thread for the full render. There are no workers, wasm threads, shared memory, or cross-origin isolation requirements.

Invalid DNA, unknown option names, wrong types, invalid values, and mismatched ensemble roles throw JavaScript `Error` objects. Timbres must be named strings; numbers and booleans must use their actual JS types. An unsupported WebAssembly environment fails during import, and browser deployment policies must allow WebAssembly compilation.

Canonical note events are deterministic. Floating-point rendering may produce different audio bytes across platforms.

## Build pipeline and package contents

```text
src/index.ts + crates/seedsong-core
  → webpack + WasmPackPlugin + wasm-pack
  → Rust wasm and generated bindings in pkg/
  → arraybuffer-loader embeds bytes; initSync initializes the module
  → webpack emits dist/index.js
  → TypeScript emits dist/*.d.ts
```

`src/index.ts` is the single entry for wasm initialization and the public API. `src/types.ts` defines public types; `src/wasm.d.ts` types the imported binary. Rust converts named request objects with `serde_wasm_bindgen::from_value` and returns metadata with `serde_wasm_bindgen::to_value`, preserving the audio data as a `Uint8Array`.

Webpack builds Rust in release mode for `build` and development mode for `build-debug`. wasm-opt is skipped, so no separate wasm-opt installation is required. The build retains dependency license notices in the output.

Only `dist/`, this README, and npm package metadata are distributed. The bundled JS contains all executable code, including wasm bytes. Generated `pkg/` files and build tools are not required by consumers.

## Tests

From the repository root:

```bash
# Build Rust/wasm and JS, check TypeScript contracts, and run Node integration tests.
npm run test:js
```

From this package directory, use `npm test`. The tests cover public exports, structured serde requests/results, DNA input formats, ensembles, validation, audio metadata, and returned memory ownership. A standalone test copies only the compiled JS file to an empty directory and disables fetch before importing it.

For manual browser tests of the Rust wasm and WebAudio implementations, follow the [project test instructions](../../README.md#manual-browser-wasm-tests).
