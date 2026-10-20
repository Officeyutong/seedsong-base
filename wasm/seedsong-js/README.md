# seedsong-js

A small JavaScript/TypeScript library for generating music from 32-byte DNA. It wraps the Rust wasm engine and exposes named options and browser-native audio types. The distributed bundle includes its wasm program and has no runtime npm dependencies. It does not create workers or require shared memory.

The package uses one source entry, `src/index.ts`, for wasm initialization and the public API. Following the wasm build approach used by `ckb-light-client`, webpack and `WasmPackPlugin` compile the Rust crate, embed its wasm through `arraybuffer-loader`, and bundle the API. TypeScript emits declarations after webpack finishes. The Rust workspace still contains only the existing core and native crates. Rust handles composition and synthesis. `serde_wasm_bindgen::from_value` decodes camelCase request objects into strict Rust types; `serde_wasm_bindgen::to_value` returns WAV metadata, preserving the audio bytes as a `Uint8Array`. No JSON strings cross the boundary.

## Build and install

From the repository root:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
npm install
npm run build:js
npm run test:js
```

Build requires Rust, Node.js 22+, npm, and wasm-pack. `npm run build:js` builds `seedsong-js`, running webpack followed by TypeScript declaration generation. The same complete build is available from this package directory with `npm run build`. WasmPackPlugin invokes wasm-pack, which builds Rust and generates matching bindings. There is no custom script manually spawning Cargo and wasm-bindgen. The public package emits a self-contained ESM `dist/index.js` plus TypeScript declarations. No publishing occurs.

Use `npm run build:js:debug` for development builds. `npm run build --workspaces` is equivalent to `npm run build:js`. `package-lock.json` is ignored by this repository; use `npm install`, not `npm ci` on a fresh checkout. To use the built package in another project:

```bash
npm install /absolute/path/to/seedsong-base/wasm/seedsong-js
```

Package layout:

```text
wasm/seedsong-js/
  src/index.ts          # Single entry: wasm initialization and public API
  src/types.ts          # Public JavaScript-facing types
  src/wasm.d.ts         # Type declaration for imported wasm bytes
  webpack.config.cjs    # Builds Rust, embeds wasm, and bundles the API
  tsconfig.json         # Type checking and declaration generation
  pkg/                  # Generated wasm-bindgen output (ignored)
  dist/                 # Self-contained public JS library and declarations (ignored)
```

Only `dist/` and the README are included in the npm package. Neither the generated intermediate files nor a second npm package are required by consumers.

The package exports only its main entry point. Import from `seedsong-js`; generated bindings and wasm memory are not public APIs. All executable code, including the wasm bytes, is bundled into `dist/index.js`. Consumers do not need Rust, wasm-pack, a separate wasm asset, an asset URL, fetch configuration, or a manual initialization call. The same ESM bundle works in browsers and Node.js.

## Render a WAV

```typescript
import { createSeedsong } from 'seedsong-js';

const music = await createSeedsong();
const dna = music.randomDna();
const wav = music.renderWav({
  dna,
  timbre: 'piano',
  sampleRate: 44100,
  loop: false,
  reverb: 0.18,
});

// wav.data is an owned Uint8Array, not a borrowed view into wasm memory.
const blob = new Blob([wav.data], { type: 'audio/wav' });
const url = URL.createObjectURL(blob);
const audio = new Audio(url);
// Run play() in a user gesture handler when required by the browser.
await audio.play();
// When finished, release the object URL with URL.revokeObjectURL(url).
```

For a download, assign the object URL to an anchor's `href` and set its `download` attribute. Revoke the URL after it is no longer needed. Save `wav.dna`, the render options, and all participant DNA to reproduce a result.

## Render for WebAudio playback

```typescript
import { createSeedsong } from 'seedsong-js';

const music = await createSeedsong();
const context = new AudioContext();
const buffer = music.renderAudioBuffer(context, {
  dna: music.randomDna(),
  timbre: 'piano',
  loop: true,
});

// In a user click handler:
await context.resume();
const source = context.createBufferSource();
source.buffer = buffer;
source.loop = true;
source.connect(context.destination);
source.start();

// Stop playback when appropriate:
source.stop();
source.disconnect();
await context.close();
```

The caller owns the context and playback sources. The library never resumes or closes a context, connects audio nodes, or starts playback. Standard WebAudio methods cover those operations, so the wrapper adds no player class or manual Rust `free()` calls. Cache an `AudioBuffer` for repeated playback. Rendering with `loop: true` wraps tails into the beginning; set the source's `loop` property as well to repeat it.

## Public API

The only runtime export is:

```typescript
createSeedsong(): Promise<Seedsong>
```

The embedded wasm is initialized synchronously once when the bundle is imported, using `initSync`. `createSeedsong()` returns the shared library object, so existing `await createSeedsong()` usage remains valid. It takes no configuration and creates no AudioContext. A browser must allow WebAssembly compilation in its Content Security Policy.

Node.js needs no wasm file loading code:

```javascript
import { writeFile } from 'node:fs/promises';
import { createSeedsong } from 'seedsong-js';

const music = await createSeedsong();
const wav = music.renderWav({ dna: music.randomDna(), timbre: 'piano' });
await writeFile('song.wav', wav.data);
```

The returned `Seedsong` object has only these methods:

| Method | Result | Purpose |
| --- | --- | --- |
| `randomDna()` | `string` | Generate 32 cryptographically random bytes as canonical lowercase hex |
| `renderWav(options)` | `RenderedWav` | Generate a complete stereo PCM16 WAV and metadata |
| `renderAudioBuffer(context, options)` | `AudioBuffer` | Generate stereo audio for a caller-owned `AudioContext` |

`RenderedWav` contains `data: Uint8Array`, `dna: string`, `sampleRate: number`, `channels: 2`, and `durationSeconds: number`. Duration includes the natural release tail unless `loop` is true. Returned bytes remain valid after subsequent renders and wasm memory growth.

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

| Field | Meaning and default |
| --- | --- |
| `dna` | Required conductor DNA: 64 hex digits, optionally prefixed with `0x` and separated by whitespace, underscores, or hyphens; or exactly 32 bytes in a `Uint8Array` |
| `timbre` | `"spec"` (default) preserves DNA timbres; `"piano"` replaces only the melody sound |
| `sampleRate` | Integer Hz between 22050 and 96000; defaults to 44100 for WAV and `context.sampleRate` for AudioBuffer |
| `loop` | Defaults to false; true produces exactly 16 bars and wraps release/reverb tails |
| `reverb` | Finite number between 0 and 1; defaults to 0.18 |
| `participants` | Optional ensemble parts; absent parts use the conductor DNA |

In an ensemble, D[16] of each participant must correspond to its part (melody 0, bass 1, pad 2, drums 3, modulo 4). The conductor's own part cannot be replaced. Its key, mode, tempo, form, harmony, and swing determine the ensemble context.

Plain number arrays are not accepted as DNA. Ambiguous booleans for timbre, stringified numbers, invalid values, and unknown option/part names are rejected. Optional fields set to `undefined` are treated as omitted. Rendering throws JavaScript `Error` objects; an unsupported WebAssembly environment fails during module import. Both rendering methods are synchronous and occupy the current JS thread while generating the full piece. There are no worker or multithreading requirements. Floating-point audio may differ across platforms even though the canonical note sequence is deterministic.

## Tests

```bash
# Builds the actual wasm, checks TypeScript contracts, and tests the JS facade in Node.
npm run test:js

# Manually inspect all Rust wasm/browser tests on one browser page.
NO_HEADLESS=1 cargo test -p seedsong-core \
  --target wasm32-unknown-unknown --release --lib
```

Node tests exercise the real wasm and serde boundary, including returned typed arrays, defaults, byte/hex input equivalence, ensembles, validation errors, restricted exports, and memory ownership. A separate import test disables fetch and verifies that the distributed bundle initializes and generates DNA without external assets. Browser tests additionally cover the actual WebAudio API. Native tests remain `cargo test --workspace`.
