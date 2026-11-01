# seedsong-js

Decode DOB/0 traits and DOB/1 SVG covers, and generate music from 32-byte DNA in JavaScript or TypeScript. The library provides WAV files and WebAudio buffers through a small typed API. Its ESM bundle includes the Rust wasm program and has no runtime npm dependencies.

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

The default entry exports:

```typescript
createSeedsong(): Promise<Seedsong>
```

It takes no arguments and returns the shared library object. The wasm module is initialized once, synchronously during import, using `initSync`. Creating the library does not create an AudioContext or start playback.

| Method | Return type | Purpose |
| --- | --- | --- |
| `decodeDob0({ dna, pattern })` | `DobOutput[]` | Decode standard DOB/0 traits in Rust |
| `decodeDob1({ traits, pattern })` | `DobOutput[]` | Append SVG images using a DOB/1 pattern in Rust |
| `generateMusic(options)` | `GeneratedMusic` | Generate deterministic events, synthesis selectors, source DNA, and SNG1 without PCM synthesis |
| `randomDna()` | `string` | Generate cryptographically random DNA as 64 lowercase hex digits |
| `renderWav(options)` | `RenderedWav` | Generate a complete stereo PCM16 WAV with metadata |
| `renderAudioBuffer(context, options)` | `AudioBuffer` | Generate stereo audio for a caller-owned `AudioContext` |

Public TypeScript types also describe DOB patterns, standard DOB outputs, music events, generation options, and render results. See `dist/types.d.ts` for the full contract.

### Render options

```typescript
interface RenderOptions {
  dna: string | Uint8Array;
  musicVersion?: 1 | 2;
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
| `musicVersion` | Deterministic note algorithm; 1 reproduces legacy music | `2` |
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
  readonly musicVersion: 1 | 2;
  readonly sampleRate: number;
  readonly channels: 2;
  readonly durationSeconds: number;
}
```

`dna` is the normalized conductor DNA. Save `musicVersion` with DNA and render settings: v2 changes melody pitches, while explicit `musicVersion: 1` retains the original output. The cover traits and DNA byte layout are unchanged. Duration includes the natural release tail unless `loop` is true. The output always contains stereo PCM16 WAV bytes, suitable for a Blob or a file write.

## DOB/0 and DOB/1

Pass the JSON arrays from the Cluster's decoder configuration, not a JSON string
or the entire Cluster description. The arrays in [pattern.md](../../pattern.md)
are the Seedsong collection's patterns. Patterns are supplied at runtime and
are not hardcoded into this library.

```typescript
import { createSeedsong } from 'seedsong-js';
import type { Dob0Pattern, Dob1Pattern } from 'seedsong-js';

const library = await createSeedsong();
const dob0Pattern: Dob0Pattern = [
  ['Color', 'String', 0, 1, 'options', ['red', 'blue']],
  ['Tempo', 'Number', 1, 1, 'range', [60, 141]],
];
const dob1Pattern: Dob1Pattern = [
  ['IMAGE.0', 'attributes', '', 'raw', "xmlns='http://www.w3.org/2000/svg'"],
  ['IMAGE.0', 'elements', 'Color', 'raw', "<circle r='20' fill='{value}'/>"],
];
const traits = library.decodeDob0({ dna: '0100', pattern: dob0Pattern });
const decoded = library.decodeDob1({ traits, pattern: dob1Pattern });
const svg = decoded.find(item => item.name === 'IMAGE')?.traits[0]?.SVG;
```

Standard output is an ordered array such as
`[{ name: 'Tempo', traits: [{ Number: 60 }] }]`; it preserves duplicate names.
DOB/1 retains public traits and appends `{ name: 'IMAGE', traits: [{ SVG: '…' }] }`
entries. Consecutive equal full image names form one SVG; `IMAGE.0` and
`IMAGE.1` produce separate outputs named `IMAGE`. Traits starting with `_` are
available during SVG assembly and then removed from the final output.

DOB/0 supports `options`, `range`, `rawNumber`, `rawString`, and `utf8`.
Numeric segments are 1–8 bytes, little-endian. Segment boundaries are clamped
at the DNA length; UTF-8 strips trailing zero bytes. Range upper bounds are
exclusive. Unlike music DNA, DOB DNA may contain any positive number of bytes.
DOB/1 supports raw fragments, `{value}` string substitution, exact string or
numeric conditions, inclusive numeric ranges, and the `["*"]` wildcard.
Unmatched conditions skip fragments; a missing named trait ends its image group,
following the official decoder's behavior. SVG attribute spaces and fragment
order are preserved. Collection fragments are markup and are not sanitized.
Use an image element or a sandbox when displaying untrusted SVG.

JS inputs and outputs use safe numeric values. An unsigned 64-bit result that
cannot be represented exactly as a JS number throws instead of silently
rounding; the native Rust API supports the full `u64` range. The APIs use
`serde_wasm_bindgen` and return ordinary objects and arrays, not ES Maps.

The implementation follows the official
[DOB/0 decoder](https://github.com/sporeprotocol/spore-dob-0/blob/master/src/decoder/mod.rs)
and [DOB/1 decoder](https://github.com/sporeprotocol/spore-dob-1/blob/master/src/decoder/mod.rs).
Local regression tests are included; running the official decoder service in
CI for independent parity verification remains a separate M2 task.

## Generate music without rendering audio

```typescript
const song = library.generateMusic({
  dna: new Uint8Array(32),
  musicVersion: 2,
  // participants: { bass: bassDna }, // Same conductor/role rules as WAV rendering.
});
console.log(song.params.bpm, song.events, song.sources);
// song.sng1 is the canonical owned Uint8Array, identical to native Song::encode().
```

`GenerateOptions` accepts only `dna`, `musicVersion`, and `participants`.
The result contains the normalized DNA, version, ordered events, integer
synthesis/arrangement selectors in `params`, and all four source DNAs.
Event start and duration are sixteenth-note steps. One step lasts
`60 / bpm / 4` seconds before the specification's swing adjustment. Pitches
are MIDI note numbers (drums use 36/38/42); parameters and source DNA determine
the timbres and other synthesis choices defined in [spec.md](../../spec.md).
No AudioContext or PCM synthesis is needed for this call. The dynamically
loaded WASM contains this music algorithm as well as DOB decoding; it can also
render complete audio through the existing methods.

## Load the WASM decoder from Cell data

Use the separate loader entry to avoid bundling the default WASM into an app
that retrieves it from CKB:

```typescript
import { loadSeedsong } from 'seedsong-js/loader';

// Your light-client-wasm integration resolves the configured Cell and verifies
// its data hash. Decode a Cell's hexadecimal data to Uint8Array first.
async function openRecord(cellData: Uint8Array<ArrayBuffer>, dna: string) {
  const library = await loadSeedsong(cellData);
  const song = library.generateMusic({ dna });
  const wav = library.renderWav({ dna, musicVersion: song.musicVersion });
  return { library, song, wav };
}
```

`loadSeedsong(input): Promise<Seedsong>` accepts an `ArrayBuffer`, a
`Uint8Array<ArrayBuffer>` (including subarrays), or a compiled
`WebAssembly.Module`. It performs no fetching and accepts no URL. Every call
creates isolated bindings, memory, and library state. Cache the returned
library per decoder Cell. The default `createSeedsong()` remains a shared,
embedded instance and is unaffected by dynamic instances.

Deploy the **raw `dist/decoder.wasm` bytes** as Cell data. Do not store the JS
bundle or its base64 representation in that Cell. Use the loader from the
**same build**: wasm-bindgen's generated import names are build-specific.
Decoder ABI version 1 describes the public contract, not binary compatibility
between arbitrary Rust/wasm-bindgen builds. Missing required exports, invalid
WASM, incompatible imports, and unsupported ABI versions reject the promise.
The loader is specific to this decoder ABI, not an arbitrary DOB WASM runner.

The loader embeds its generated JS glue as text, but no WASM. It imports an
isolated glue module through a temporary Blob URL in browsers/Workers, revoked
after initialization. A CSP must permit `blob:` module scripts and WASM
compilation (`wasm-unsafe-eval` where supported). Node uses an isolated `data:`
module URL. This mechanism needs neither `eval` nor a server for JS glue.

This release keeps the audio renderer in the same WASM to preserve the existing
API and uses Rust `std`. **The original M2 ≤60 KB requirement will not be met
in this iteration.** We propose accepting the larger complete decoder rather
than retaining 60 KB as an outstanding delivery promise. This scope adjustment
has not been approved by the funding committee. See the
[scope adjustment and forum draft](../../reports/m2/scope-adjustment.md).
The decoder has been [deployed on CKB testnet inside a ZIP archive](../../reports/m2/README.md).
Consumers must unzip the Cell data before passing WASM bytes to this loader.
Each build's actual binary size and checksum are recorded in `dist/decoder.json`;
use the archived loader and metadata when loading that deployed version.

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

Decoding, event generation, and both render methods are synchronous and occupy the current JS thread for the full render. There are no workers, wasm threads, shared memory, or cross-origin isolation requirements.

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

`src/index.ts` initializes embedded WASM. `src/loader.ts` initializes caller-supplied WASM. Both use `src/api.ts` for the public facade. `src/types.ts` defines public types; `src/wasm.d.ts` types the imported binary. Rust converts named request objects with `serde_wasm_bindgen::from_value` and returns metadata with `serde_wasm_bindgen::to_value`, preserving the audio data as a `Uint8Array`.

Webpack builds Rust in release mode for `build` and development mode for `build-debug`. wasm-opt is skipped, so no separate wasm-opt installation is required. The build retains dependency license notices in the output.

Only `dist/`, this README, and npm package metadata are distributed. The bundled JS contains all executable code, including wasm bytes. The build also emits `dist/loader.js` without WASM bytes, `dist/decoder.wasm` for Cell deployment, and `dist/decoder.json` containing its size, SHA-256 and decoder ABI version. The embedded binary and emitted binary are identical. Generated `pkg/` files and build tools are not required by consumers.

## Tests

From the repository root:

```bash
# Build Rust/wasm and JS, check TypeScript contracts, and run Node integration tests.
npm run test:js
```

From this package directory, use `npm test`. The tests cover public exports, structured serde requests/results, DNA input formats, ensembles, validation, audio metadata, and returned memory ownership. A standalone test copies only the compiled JS file to an empty directory and disables fetch before importing it.

For manual browser tests of the Rust wasm and WebAudio implementations, follow the [project test instructions](../../README.md#manual-browser-wasm-tests).
