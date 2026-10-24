# M1 quality report and native test evidence

Recorded on **2026-10-24** for source revision
`9d03677b1c15b5af8f39e24ce35d8f7c5f4a200e` (`seedsong-music/1`).
The working tree was clean before these commands ran. Only evidence and
documentation were added afterward; the music algorithm was not changed.

## Outcome

The native workspace test suite passed: **12 passed, 0 failed, 0 ignored**.
The quality run completed for **1,000 distinct DNA values**, but **not all
quality targets passed**. These are separate results: passing implementation
tests does not establish that every generated melody meets the statistical
quality targets in [specification Section 12](../../spec-v1.md#12-quality-checks).

This directory supplies the batch report and native test evidence for M1. It
does not establish completion of the entire milestone. Human listening and
verification of the public Cloudflare Pages demo are not included in this run.

## Files

| File | Contents |
| --- | --- |
| [quality-report.json](quality-report.json) | Unmodified CLI output: all 1,000 DNA values, per-song metrics, and aggregate results |
| [workspace-tests.log](workspace-tests.log) | Captured Cargo test completion output, including every executed test and harness result |
| [evidence.json](evidence.json) | Source revision, toolchain, commands, exit codes, aggregate findings, and artifact SHA-256 hashes |

## Batch generation and measurements

The existing `--check` command generated DNA using `Stream::new("sng/quality",
index.to_le_bytes())` for each index from 0 through 999. Four consecutive
64-bit outputs, each encoded little-endian, make each 32-byte DNA. This fixed
pseudorandom corpus is reproducible; it is not a hand-selected set of songs.
Every DNA is unique within the corpus.

Measurements cover the complete solo melody event sequence, including phrase
boundaries. Intervals are measured in scale degrees, and a large leap is an
adjacent interval of at least four scale degrees. Repetition counts are
overlapping windows of three identical consecutive pitches; they are not
counts of affected songs. These checks inspect events, not rendered audio.

| Metric | Actual result | Assessment |
| --- | --- | --- |
| Non-chord melody tones on strong beats | 0 | Pass |
| Melody notes outside the specified range | 0 | Pass |
| Final melody note is the tonic | 1,000 / 1,000 | Pass |
| Melody note count within 40–160 | All songs; observed range 42–121 | Pass |
| Large-leap proportion across the complete corpus | 3,403 / 79,483 intervals = 4.2814% | Below 8% in aggregate |
| Maximum individual-song large-leap proportion | 14 / 63 intervals = 22.2222% | Fails the report generator's per-song 8% check |
| Songs exceeding an individual 8% large-leap proportion | 166 / 1,000 | Requires review |
| Three-identical-pitch windows | 999 windows in 520 songs | Fails the zero-repetition target |
| Songs failing at least one report target | 628 / 1,000 | Not an all-pass corpus |
| CLI `summary.all_targets_pass` | `false` | Quality targets are not fully met |

The specification states an 8% large-leap target without explicitly separating
corpus-wide and individual-song evaluation. The current report generator uses
the stricter **maximum individual-song ratio** in `all_targets_pass`. Both
results are disclosed above; the aggregate passing ratio must not be presented
as every song passing. Repeated pitches independently fail the stated target.

The report command exits successfully after writing its observations even when
`all_targets_pass` is false. Its exit code is not a quality acceptance gate.

### Samples for follow-up listening

| Issue | DNA | Observation |
| --- | --- | --- |
| Highest large-leap ratio | `5bf48d079c2680dd2a20fa36cb9e699caf567174410030c762975c0ff503ee31` | 14 / 63 intervals; 22.2222% |
| Next highest large-leap ratio | `ee2f6f7f115051c259baa6c26319f920434e31a41ff084875169a8dce5010353` | 10 / 51 intervals; 19.6078% |
| Most repeated-pitch windows | `2aca91c192c493123d79aeb89ddab79e77f1cbf432df3c52339cbece0d26cabe` | 8 windows |

These samples were identified automatically and **have not been manually
listened to as part of this evidence collection**. The existing README notes
that chord snapping and phrase copying can introduce repetitions and large
leaps. Establishing the cause in each sample and changing the rules require
follow-up work; the measurements were not filtered or repaired to hide failures.

## Native workspace tests

| Harness | Passed | Failed | Ignored |
| --- | --- | --- | --- |
| Core library unit tests | 2 | 0 | 0 |
| Audio integration tests | 2 | 0 | 0 |
| Core/backend integration tests | 8 | 0 | 0 |
| Native CLI unit-test harness | 0 | 0 | 0 |
| Core documentation-test harness | 0 | 0 | 0 |
| Total | 12 | 0 | 0 |

Coverage includes DNA parsing, the published PRNG vector, musical invariants,
100 existing music regression vectors, ensemble behavior, backend contracts,
WAV encoding, exact loop length, and identical native rendering with one versus
four Rayon threads. The 100 regression vectors are separate from the 1,000-DNA
quality corpus and are not independent acceptance vectors.

The command was `cargo test --workspace`. Plain `cargo test` at the repository
root selects only the default native crate, so it is insufficient to reproduce
this full test run. Browser WASM tests and JS/UI tests are outside this native
test result and were not run for this evidence collection.

## Reproduce

Run from the repository root using Bash. A fresh temporary directory avoids the
CLI's intentional refusal to overwrite existing reports:

```bash
set -o pipefail
SEEDSONG_M1_RUN_DIR="$(mktemp -d)"

cargo test --workspace 2>&1 | tee "$SEEDSONG_M1_RUN_DIR/workspace-tests.log"

cargo run --release -- --music-version 1 --check 1000 \
  --report "$SEEDSONG_M1_RUN_DIR/quality-report.json" --threads 4

cargo run --release -- --music-version 1 --check 1000 \
  --report "$SEEDSONG_M1_RUN_DIR/quality-report-1-thread.json" --threads 1

cmp reports/m1/quality-report.json "$SEEDSONG_M1_RUN_DIR/quality-report.json"
cmp "$SEEDSONG_M1_RUN_DIR/quality-report.json" \
  "$SEEDSONG_M1_RUN_DIR/quality-report-1-thread.json"
```

The one-thread and four-thread reports were compared in this run and are
byte-for-byte identical. Cargo test ordering, timings, and build messages may
vary, so the test log itself is not expected to reproduce byte-for-byte.

Environment: Linux x86_64; `rustc 1.99.0 (b940084d7 2026-09-28)`;
`cargo 1.99.0 (5f94df478 2026-08-27)`.
