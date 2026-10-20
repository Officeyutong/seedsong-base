# Seedsong Rust

Decode a 32-byte DNA into 16 bars of music according to [spec.md](spec.md), synthesize stereo PCM WAV in Rust, and optionally convert it with ffmpeg based on the output extension. All composition logic uses integers. Rayon parallelizes part generation, note synthesis, mixing in blocks, normalization, and DNA quality checks.

Requires Rust with edition 2024 support. WAV output needs no external software; other formats require `ffmpeg` on PATH.

```bash
cargo build --release
# Run without arguments: paste DNA, choose parameters step by step, or randomize everything
cargo run --release
# Random DNA, piano melody, Opus output; keep the intermediate WAV
cargo run --release -- --random --timbre piano -o song.opus
# Supply a complete DNA and export canonical SNG1 events alongside the audio
cargo run --release -- \
  --dna 9a401c07e8553fa2710c0000000000000209b62d000000000000000000000000 \
  --timbre piano --threads 4 -o song.wav --events song.sng
# Generate events only, without synthesizing audio
cargo run --release -- --random --events-only --events song.sng
# An exact 16-bar loop, with audio tails wrapped to the beginning
cargo run --release -- --random --loop --reverb 0.12 -o loop.flac
```

Interactive mode prompts for mode, tempo, chord progression, contour, timbres, key, form, arrangement, swing, density, and seeds. Press Enter or type `r` to randomize any choice. DNA input accepts a `0x` prefix, spaces, underscores, and hyphens, but must contain exactly 64 hexadecimal characters after removing separators. Non-interactive use requires `--dna` or `--random`.

The default output is WAV. Supported extensions are `wav`, `flac`, `opus`, `ogg` (Opus encoding), `mp3`, `m4a`, and `aac`. Opus is suitable for sharing; FLAC preserves lossless audio. Conversion keeps the intermediate WAV, and reports its location if ffmpeg is unavailable or fails. By default, audio includes its natural release tail. `--loop` wraps release and reverb tails to the beginning; codec delay in lossy formats may affect looping in some players.

Each audio export also saves `.dna.txt` and `.json` sidecars with the same stem, recording the complete DNA, ensemble participants, and rendering parameters. Existing files are never overwritten. Supported sample rates are `22050/44100/48000/96000`; `--threads` controls parallelism, and `--reverb 0` disables reverb.

## Workspace and backends

The project contains two crates:

```text
crates/
  seedsong-core/    # DNA, note events, synthesis, WAV encoding, and the music backend trait
  seedsong-native/  # Native CLI, interactive input, validation, and quality reports
```

The root is a Cargo workspace with `seedsong-native` as its default member. The executable remains `seedsong-base`, so existing `cargo run --release -- ...` commands still work. Native file I/O, ffmpeg, and Rayon are isolated by target. The core library does not depend on clap, and wasm builds do not include Rayon.

`MusicBackend` defines `render(&mut self, &Song, Options)`, with associated types for output and errors. `generate` decodes the DNA and invokes the backend. On native targets, `NativeBackend` returns `PcmAudio`, which supports `write_wav` and in-memory `to_wav`. On wasm, `WebAudioBackend` returns a WebAudio `AudioBuffer` and provides playback controls. Both targets share synthesis and mixing algorithms. Wasm uses ordinary iterators and requires no threads, shared memory, or cross-origin isolation headers.

```rust
use seedsong_core::{generate, dna::Dna, synth::Options,
    backend::native::NativeBackend};

let audio = generate(&mut NativeBackend, Dna([0; 32]), [None; 4], Options::default())?;
audio.write_wav(std::path::Path::new("song.wav"))?;
// Use inside a function returning anyhow::Result<()>.
```

## JavaScript library / WebAudio

The TypeScript facade in [wasm/seedsong-js](wasm/seedsong-js/README.md) follows the separate wasm-bindings and public-wrapper structure used by ckb-light-client. It exposes only initialization, random DNA, WAV rendering, and AudioBuffer rendering. Internal Rust backends, manual memory management, and SNG1 encoding are not exported as public JavaScript APIs.

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --locked
npm install
npm run build:js
npm run test:js
```

The `seedsong-js` package has one source entry, `src/index.ts`. Webpack + WasmPackPlugin compiles Rust, embeds the wasm through arraybuffer-loader, and bundles initialization together with the public API; TypeScript then emits the declarations. Consumers do not need a separate wasm file or Rust build tools. Install the built package from `wasm/seedsong-js` in your application. See the package README for the build pipeline, Node.js, ensembles, errors, and the full API contract. Use `npm run build:js:debug` for development builds. `package-lock.json` is ignored; a fresh checkout uses `npm install`.

```typescript
import { createSeedsong } from 'seedsong-js';

const music = await createSeedsong();
const dna = music.randomDna();
const wav = music.renderWav({ dna, timbre: 'piano', sampleRate: 44100 });
const blob = new Blob([wav.data], { type: 'audio/wav' });

const context = new AudioContext();
const buffer = music.renderAudioBuffer(context, { dna, timbre: 'piano', loop: true });
// In a user click handler:
await context.resume();
const source = context.createBufferSource();
source.buffer = buffer;
source.loop = true;
source.connect(context.destination);
source.start();
// When finished: source.stop(); source.disconnect(); await context.close();
```

Options use named JavaScript fields and `"spec" | "piano"` timbres. DNA is a hex string or 32-byte `Uint8Array`; WAV output contains owned audio bytes and metadata. `serde_wasm_bindgen` handles structured Rust/JS conversion, without JSON strings. Rendering occupies the current JS thread and uses no wasm multithreading. The caller owns WebAudio playback and context lifetime. Browser WAV generation does not call ffmpeg.

### Viewing wasm tests manually in a browser

All browser tests share one library test entry point: DNA/RNG, backend interfaces, music algorithms, wasm/WAV helpers, structured JS options, and WebAudio. Start one server and view all results on one page:

```bash
NO_HEADLESS=1 cargo test -p seedsong-core \
  --target wasm32-unknown-unknown --release --lib
```

Open the URL printed in the terminal, usually `http://127.0.0.1:8000`. The page runs the tests and displays their results. The server stays running; refresh the page to rerun the tests, and press Ctrl-C in the terminal when finished. This requires neither ChromeDriver nor manually generated JS bindings, only the wasm target and a matching version of wasm-bindgen-cli.

Filter by module name to inspect a particular suite manually:

```bash
# View only the music algorithm tests
NO_HEADLESS=1 cargo test -p seedsong-core \
  --target wasm32-unknown-unknown --release --lib music_tests

# View only the WebAudio tests
NO_HEADLESS=1 cargo test -p seedsong-core \
  --target wasm32-unknown-unknown --release --lib web_audio_tests
```

`.cargo/config.toml` configures wasm-bindgen-test-runner. For test builds, `src/lib.rs` loads the `run_in_browser` configuration and the modules under `tests/suites/` into a single wasm test program. Previously, each integration test was a separate program, and interactive mode stayed on the first one, showing only the two DNA/RNG tests. All suites now share the same server and page.

Run native tests with:

```bash
cargo test --workspace
```

Thread consistency and file I/O tests run only on native targets. `tests/core.rs` reuses the backend and music assertions to avoid maintaining duplicate tests.

To compile wasm tests without running them:

```bash
cargo test -p seedsong-core --target wasm32-unknown-unknown --release --lib --no-run
```

The tests cover all 100 SNG1 regression vectors, random DNA, invalid input, WAV output for both timbres, normalized peaks, exact loop length, and real WebAudio source controls. WebAudio tests schedule sources while the context is suspended, so they do not require autoplay permission.

## Timbres and parallelism

`--timbre spec` uses the six lead timbres, three bass timbres, pad, and drums selected by the DNA. `--timbre piano` replaces only the melody's sound, preserving its events and the other parts. The piano model uses up to 24 inharmonic sinusoidal partials, slightly detuned paired strings, two-stage exponential decay, and brief hammer noise. Register parameters are interpolated, and sine oscillators use a 16,384-point table with linear interpolation. This is an approximate piano sound that requires no sample library.

The piano model draws on the inharmonicity, partial decay, and beating described in [Physics-informed differentiable method for piano modeling](https://www.frontiersin.org/journals/signal-processing/articles/10.3389/frsip.2023.1276748/full), and the sinusoidal/transient-noise decomposition in [Sines, Transient, Noise Neural Modeling of Piano Notes](https://arxiv.org/abs/2409.06513). Parameters in this implementation are set by hand; it does not use the papers' trained models.

Notes are synthesized independently in parallel, then mixed in parallel output blocks. Each block accumulates events in a fixed order without a shared mixer lock. The normalized peak is 0.95. Tests verify identical floating-point audio with 1 and 4 threads on the same machine. Floating-point rendering and lossy encoding do not promise byte-for-byte reproducibility across platforms; the normative output is `.sng`.

## Ensembles

Use `--dna` for the conductor and repeat `--part melody=DNA`, `--part bass=DNA`, `--part pad=DNA`, or `--part drums=DNA` for participants. Each participant's D[16] must match its part. The conductor's own part cannot be replaced, and unspecified parts use the conductor's DNA. The conductor controls mode, key, tempo, form, harmony, and swing. Other parameters and timbres come from each part's DNA.

## Validation and specification boundaries

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run --release -- --check 1000 --report another-report.json --threads 4
```

[quality-report.json](quality-report.json) contains complete results for 1,000 DNA values generated from fixed seeds: 0 non-chord tones on strong beats, 0 out-of-range notes, tonic endings in 1,000/1,000 songs, and 40–160 melody notes in every song. Large leaps account for 4.28% of intervals overall, with a maximum of 22.22% in an individual song. There are 999 windows of three consecutive repeated notes. These observations result from following the snapping, range, and phrase-copying rules. Not all quality targets in Section 12 pass; the implementation does not silently add corrections that would change the output defined in Section 11.

The repository originally supplied no independent acceptance vectors. [vectors/music-v1.json](vectors/music-v1.json) contains 100 local regression vectors generated by this implementation. They detect future behavioral changes but do not prove byte-for-byte compatibility with other implementations. Independent vectors still need to be checked when available.

The specification leaves some edge cases open. This implementation fixes the following interpretations for comparison with independent implementations:

- `last_iv` records the actual scale-degree interval after snapping. The initial `prev = 0` does not count as a previously played note.
- Ties in range correction and tonic endings choose the lower pitch. Moving toward the center means one degree downward when already exactly at the center.
- Repeated phrase letters copy pitch and duration for the first three bars. Velocities are drawn again from the human stream in output order.
- The human stream assigns all melody velocities first, then hi-hat velocities in time order. In an ensemble, a drum part retains its DNA's stream position in the conductor's context, so its melody note count is calculated first, but only drum events are emitted.
- When C first appears as the final phrase of ABAC, its original fourth bar is generated first. The ending bar is then regenerated with the vary stream, restoring the history at the end of the third bar.
- The density offset for B/C is `min(1, DNA_offset + 1)`.

These interpretations form the current regression baseline. Cross-implementation acceptance is not claimed until independent vectors have been checked.
