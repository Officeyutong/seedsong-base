# Proposed M2 decoder size adjustment

Status: project-side decision recorded on 2026-11-01; funding committee
agreement has not been recorded. This document is not an approved amendment
or a claim that M2 is complete.

## Decision and measured size

We will not meet the original M2 decoder limit of 60 KB in this iteration.
We propose removing that size limit from the current delivery commitment and
deploying the larger complete WASM decoder on the CKB testnet.

The local release artifact inspected on 2026-11-01 is **384,498 bytes**
(approximately 375.5 KiB), measured on the raw `dist/decoder.wasm`, not on
compressed transfer bytes or the JS bundle. Its SHA-256 is:

```text
69df207a3e6ebfc5bf98f46ef26aabe3ebf2266bd9d212ea8eb374f79a8706c0
```

This is one measured build, not a frozen artifact or a promise that future
builds will have the same size. Each build emits its actual size and SHA-256
in `wasm/seedsong-js/dist/decoder.json`.

The current binary includes DOB/0 and DOB/1 interpretation, deterministic
music generation, PCM/WAV synthesis, and the wasm-bindgen bridge. It uses Rust
`std`; the originally proposed `no_std` implementation has not been delivered.
We are retaining this implementation for the current iteration. This decision
does not establish that a different implementation could never fit in 60 KB.

## What remains required

- Store the complete decoder, including the music generation algorithm, in the
  on-chain ZIP archive. Extract the raw WASM entry before execution. Keeping only DNA on-chain and fetching the algorithm
  from an external server would not satisfy this design.
- Allow the JS loader to instantiate the WASM bytes retrieved from that Cell.
- Publish the actual deployed artifact size, hash, Cell location and transaction.
  A larger decoder increases occupied capacity; deployment documentation must
  report the actual transaction's capacity requirements.
- Create the Cluster, register its patterns and official decoder references,
  mint at least 50 test records, verify cover and trait display, and implement
  the official SVG comparison CI required by M2.

Future size optimization may be investigated, but 60 KB is no longer a
project-side promise for this iteration. Until the committee agrees to this
adjustment, the original proposal's size criterion remains an unmet requirement.

## Suggested public progress update

> We need to adjust the WASM size commitment in M2. The original proposal set
> a limit of 60 KB; we will not achieve that limit in this iteration. Our current
> complete release decoder is 384,498 bytes (about 375.5 KiB). It includes DOB/0
> and DOB/1 decoding, deterministic music generation, and audio synthesis, and
> uses Rust std rather than the originally proposed no_std implementation.
>
> We propose deploying this larger decoder on the testnet and reporting its
> actual size, hash, occupied capacity and deployment transaction. The music
> algorithm will still be stored on-chain, and the browser will execute the WASM
> retrieved from the Cell. The other M2 deliverables remain in scope: Cluster
> configuration, at least 50 test records, cover/trait display verification and
> official decoder output comparison in CI. Please confirm whether this scope
> adjustment is acceptable. We are not claiming that M2 is complete or that the
> original size requirement has been satisfied.
