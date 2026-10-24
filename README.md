# Seedsong

Generate 16 bars of music from a 32-byte DNA. Seedsong implements the deterministic note-generation rules in [spec.md](spec.md), synthesizes stereo audio in Rust, and provides a native command-line program and a JavaScript/TypeScript library.

The note event sequence is the reproducibility boundary: the same DNA and music version produce the same SNG1 events. Audio rendering uses floating-point arithmetic and may differ across platforms.

[pattern.md](pattern.md) specifies the DOB/0 musical and cover traits, the DOB/1 SVG cover pattern, and Cluster metadata for the proposed on-chain collection. These patterns are a specification; Cluster deployment and browser cover restoration are not implemented by the current audio library.

The current algorithm is **seedsong-music/2**. It refines complete melody paths
with chord, range, repetition and leap-budget constraints while preserving the
v1 rhythm, velocities and accompaniment. Use `--music-version 1` or JS
`musicVersion: 1` to reproduce legacy output; always save the music version with
DNA. The original rules remain in [spec-v1.md](spec-v1.md).

## Project structure

```text
crates/
  seedsong-core/          # DNA decoding, events, synthesis, WAV encoding, and backends
  seedsong-native/        # Interactive CLI, file export, and quality reports
wasm/
  seedsong-js/
    src/index.ts         # Single JS entry: wasm initialization and public API
    src/types.ts         # Public TypeScript types
    src/wasm.d.ts        # Type declaration for imported wasm bytes
    webpack.config.cjs   # Rust build, wasm embedding, and JS bundling
    pkg/                 # Generated wasm bindings and binary (ignored)
    dist/                # Bundled JS and type declarations (ignored)
vectors/music-v1.json    # Legacy regression vectors
vectors/music-v2.json    # Current regression vectors
spec.md                  # Music generation specification
pattern.md               # DOB/0 traits, DOB/1 cover pattern, and Cluster metadata
```

The Cargo workspace defaults to `seedsong-native`, whose executable is named `seedsong-base`. The npm workspace contains one package, `seedsong-js`.

## Requirements

| Task | Requirements |
| --- | --- |
| Native CLI or Rust library | Rust with edition 2024 support |
| Native export to compressed audio | `ffmpeg` on PATH |
| Build the JS library | Rust, `wasm32-unknown-unknown`, wasm-pack, Node.js 22+, and npm |
| Use the built JS library | An ESM environment with WebAssembly; WebAudio playback also needs a browser `AudioContext` |
| Run Rust wasm tests manually | The wasm target, matching wasm-bindgen-cli, and a browser |

Native WAV generation and JS WAV generation do not require ffmpeg.

## Native CLI

Run from the repository root:

```bash
# Interactive input: paste DNA, choose parameters step by step, or randomize everything.
cargo run --release

# Random DNA and piano melody; export Opus and retain the intermediate WAV.
cargo run --release -- --random --timbre piano -o song.opus

# A fixed DNA, WAV audio, and canonical SNG1 note events.
cargo run --release -- \
  --dna 9a401c07e8553fa2710c0000000000000209b62d000000000000000000000000 \
  --timbre piano --threads 4 -o song.wav --events song.sng

# Events only.
cargo run --release -- --random --events-only --events song.sng

# An exact 16-bar loop with tails wrapped to the beginning.
cargo run --release -- --random --loop --reverb 0.12 -o loop.flac
```

Interactive prompts cover mode, tempo, chord progression, contour, timbres, role, key, form, arrangement, swing, density, and seeds. Press Enter or type `r` to randomize a choice. DNA input accepts a `0x` prefix and whitespace, underscores, or hyphens; after normalization it must contain exactly 64 hexadecimal digits. Non-interactive use requires `--dna` or `--random`.

| Option | Behavior |
| --- | --- |
| `--music-version` | Select algorithm 2 (default), or 1 for legacy reproduction |
| `--timbre spec` | Use DNA-selected timbres; this is the default |
| `--timbre piano` | Replace the melody's sound with synthesized piano |
| `--sample-rate` | Native CLI accepts 22050, 44100, 48000, or 96000 Hz; default 44100 |
| `--threads` | Set the Rayon worker count; default is available parallelism |
| `--reverb` | Set reverb from 0 to 1; default 0.18 |
| `--loop` | Wrap release and reverb tails into an exact 16-bar loop |
| `--events` | Write the canonical SNG1 binary event sequence |
| `--events-only` | Skip audio synthesis |
| `--part ROLE=DNA` | Add an ensemble participant; repeat for different parts |

Output defaults to WAV. Extensions `flac`, `opus`, `ogg` (Opus), `mp3`, `m4a`, and `aac` invoke ffmpeg. The intermediate WAV is retained, including when conversion fails. Each audio export also writes `.dna.txt` and `.json` sidecars containing the DNA, participant sources, and render settings. Existing files are not overwritten. Lossy codec delay may affect seamless looping in some players.

Build the executable with `cargo build --release`; inspect all options with `cargo run --release -- --help`.

## JavaScript / TypeScript library

Build from the repository root:

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
npm install
npm run build:js
```

For development builds, use `npm run build:js:debug`. From `wasm/seedsong-js`, `npm run build` builds the same package directly. This repository ignores `package-lock.json`, so a fresh checkout uses `npm install`.

The package has one source entry, `wasm/seedsong-js/src/index.ts`. Webpack's `WasmPackPlugin` builds `seedsong-core` through wasm-pack, `arraybuffer-loader` embeds the binary, and the entry initializes it with `initSync`. Webpack emits `dist/index.js`; TypeScript then generates the public declarations. The resulting ESM library includes its wasm and needs no external asset loading or Rust tools at runtime.

Install the built package into another application:

```bash
npm install /absolute/path/to/seedsong-base/wasm/seedsong-js
```

```typescript
import { createSeedsong } from 'seedsong-js';

const music = await createSeedsong();
const wav = music.renderWav({
  dna: music.randomDna(),
  timbre: 'piano',
  sampleRate: 44100,
});
const blob = new Blob([wav.data], { type: 'audio/wav' });
```

The sole runtime export is `createSeedsong()`. Its returned object provides `randomDna()`, `renderWav(options)`, and `renderAudioBuffer(context, options)`. Inputs use named options and accept DNA as hex or a 32-byte `Uint8Array`. Rust uses `serde_wasm_bindgen` for structured conversion and returns owned audio bytes with metadata. Rendering is synchronous on the current JS thread. The caller owns WebAudio playback and context lifetime.

See the [JS package README](wasm/seedsong-js/README.md) for the complete API, WebAudio playback, Node.js file export, ensembles, and error behavior.

## Rust library and backends

`seedsong-core` separates deterministic composition from audio rendering. `MusicBackend` defines `render(&mut self, &Song, Options)` with associated output and error types; `generate` decodes the conductor and optional participants before calling the backend.

| Target | Backend | Output and execution |
| --- | --- | --- |
| Native | `NativeBackend` | `PcmAudio`, parallel synthesis with Rayon, WAV/file export |
| wasm | `WebAudioBackend` | WebAudio `AudioBuffer`, single-threaded synthesis |

Both targets share the composition and synthesis algorithms. Native file I/O, ffmpeg, and Rayon are isolated from wasm builds. `PcmAudio::to_wav` also supports in-memory WAV encoding on wasm. The JS library exposes its typed facade rather than Rust backend handles.

In a Rust application depending on `seedsong-core` and `anyhow`:

```rust
use seedsong_core::{
    backend::native::NativeBackend,
    dna::Dna,
    generate,
    synth::Options,
};

fn main() -> anyhow::Result<()> {
    let audio = generate(&mut NativeBackend, Dna([0; 32]), [None; 4], Options::default())?;
    audio.write_wav(std::path::Path::new("song.wav"))?;
    Ok(())
}
```

## Ensembles

The conductor controls mode, key, tempo, form, harmony, and swing. Each participant supplies its own part's patterns, seeds, and timbres; missing parts use the conductor's DNA. A participant's `D[16] % 4` must match its role: melody 0, bass 1, pad/harmony 2, drums/rhythm 3. The conductor always performs its own role, which cannot be replaced.

The CLI accepts repeated `--part melody=DNA`, `--part bass=DNA`, `--part pad=DNA`, or `--part drums=DNA`. The JS library accepts a named `participants` object; see its [ensemble example](wasm/seedsong-js/README.md#ensembles).

## Tests and checks

### Native

```bash
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
```

These cover the music rules, regression vectors, backend contracts, WAV files, and consistency between native thread counts.

### JavaScript library

```bash
npm run test:js
```

This rebuilds the actual wasm and JS bundle, checks TypeScript contracts, and runs Node.js integration tests against the public API. A standalone-bundle test copies only `dist/index.js` to an empty directory and disables fetch, verifying that no external wasm asset or helper module is needed.

### Manual browser wasm tests

Install the runner matching the wasm-bindgen version in `Cargo.lock` (currently 0.2.129):

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
NO_HEADLESS=1 cargo test -p seedsong-core \
  --target wasm32-unknown-unknown --release --lib
```

Open the URL printed by the terminal, usually `http://127.0.0.1:8000`. All browser suites are included in one library test binary and appear on the same page. Refresh to rerun them; press Ctrl-C in the terminal to stop the server. Manual mode needs no ChromeDriver.

The suites cover DNA/RNG, backend contracts, music rules, WAV generation, structured JS requests, and real WebAudio operations. WebAudio tests schedule sources while the context is suspended, so they do not require autoplay permission.

Filter by module name:

```bash
NO_HEADLESS=1 cargo test -p seedsong-core \
  --target wasm32-unknown-unknown --release --lib music_tests

NO_HEADLESS=1 cargo test -p seedsong-core \
  --target wasm32-unknown-unknown --release --lib web_audio_tests
```

To check wasm compilation and linting without opening a browser:

```bash
cargo test -p seedsong-core --target wasm32-unknown-unknown --release --lib --no-run
cargo clippy -p seedsong-core --target wasm32-unknown-unknown --all-targets -- -D warnings
```

## Synthesis and reproducibility

`spec` rendering implements the DNA-selected lead and bass timbres, pad, and drums. Piano rendering replaces only the melody sound, using up to 24 inharmonic sine partials, slightly detuned paired strings, two-stage decay, and brief hammer noise. Register parameters are interpolated; sine oscillators use a 16,384-point table with linear interpolation. No sample library is required.

The model draws on [Physics-informed differentiable method for piano modeling](https://www.frontiersin.org/journals/signal-processing/articles/10.3389/frsip.2023.1276748/full) and [Sines, Transient, Noise Neural Modeling of Piano Notes](https://arxiv.org/abs/2409.06513). Parameters are set by hand rather than taken from trained models.

Native synthesis computes notes in parallel and mixes output blocks in fixed event order. The normalized peak is 0.95. Tests verify identical floating-point audio with 1 and 4 threads on the same machine. Wasm performs the same work serially, with no workers, shared memory, or cross-origin isolation requirement. Cross-platform audio bytes and lossy encodings are not the normative output; SNG1 note events are.

### Quality reports and compatibility

The [v2 quality comparison](reports/m1-v2/README.md) records the new 1,000-DNA
report, workspace tests and validation details. The original [v1 M1
evidence](reports/m1/README.md) is preserved, including its failed quality targets.
Version 2 meets all measured targets on the same corpus; this is objective
constraint validation, not a substitute for listening.

Generate a quality report for 1,000 DNA values using fixed seeds:

```bash
cargo run --release -- --check 1000 --report quality-report.json --threads 4
```

The report measures final events against specification Section 12. Version 2 enforces no triple repeated pitches and a per-song large-leap budget after chord snapping and phrase assembly. Version 1 preserves the original behavior. Generating a report returns success even if its quality targets fail; inspect `summary.all_targets_pass`.

Both [vectors/music-v1.json](vectors/music-v1.json) and [vectors/music-v2.json](vectors/music-v2.json) contain 100 local regression vectors generated by their respective algorithms. They detect behavior changes but are not independent acceptance vectors. Cross-implementation compatibility remains to be verified against an independent baseline.

The shared v1 draft generation resolves these edge cases before v2 refinement:

- `last_iv` records the interval after snapping. Initial `prev = 0` is not a previously played note.
- Range-correction and tonic-ending ties choose the lower pitch. A step toward the center goes downward when already at the center.
- Repeated phrases copy pitch and duration for the first three bars, then draw new velocities in output order.
- The human stream assigns melody velocities before hi-hat velocities. An ensemble drum part first computes its own DNA's melody note count in the conductor's context to preserve that stream position, but emits only drum events.
- For the final C in ABAC, generate its original fourth bar, restore the history at the end of the third bar, then regenerate the ending with the vary stream.
- The density offset for phrases B/C is `min(1, DNA_offset + 1)`.
