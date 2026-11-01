# M2 deployment evidence

## Testnet decoder archive

Observed on 2026-11-01 using `https://testnet.ckb.dev`: the transaction is
committed and output 0 is live.

```text
0x4d54a43844847bd053b8cedf6bf70deb5cdb5904a43378f6db7c4426887a22ba:0
```

[View transaction in CKB Testnet Explorer](https://pudge.explorer.nervos.org/transaction/0x4d54a43844847bd053b8cedf6bf70deb5cdb5904a43378f6db7c4426887a22ba).
The [machine-readable verification record](testnet-deployment.json) contains
the block reference, genesis hash, file hashes, and runtime check results.
Live status and commitment are observations at verification time.

| Property | Observed value |
| --- | --- |
| Container | ZIP, `seedsong-archive/1` manifest |
| Archive size | 159,758 bytes |
| Occupied capacity | 159,819 CKB |
| Entries | `decoder.wasm`, `loader.js`, `decoder.json`, `spec.md`, `spec-v1.md`, `pattern.md`, `bundle.json` |
| Raw WASM size | 384,498 bytes |
| Decoder ABI | 1 |

All original entry sizes, SHA-256 hashes and CKB-personalized Blake2b hashes
matched the archive manifest. The WASM size and SHA-256 also matched
`decoder.json`. The WASM and loader matched the local built artifacts
byte-for-byte.

The downloaded WASM was instantiated in Node.js using the matching local
loader, with network fetching disabled during execution. The archived pattern
and reference DNA produced an SVG cover and 367 music events using music v2;
the resulting SVG and SNG1 hashes are recorded in the JSON evidence. This check
did not run browser playback or official decoder parity tests.

The Cell contains **ZIP bytes**, not a bare WASM module. Consumers must verify
and unzip the archive, read entries by name, and pass `decoder.wasm` to the
matching loader. The music algorithm is included in the on-chain WASM.

## Remaining M2 acceptance work

- Agree the [size adjustment](scope-adjustment.md) with the funding committee.
  ZIP compression does not satisfy the original 60 KB raw decoder requirement.
- Create and document the Spore Cluster with official DOB/0 and DOB/1 decoder
  references, patterns, minting permissions and the archive reference.
- Mint at least 50 test records and publish the transaction references.
- Verify cover and trait display in the intended compatible wallet/explorer.
- Add CI comparison against the official decoder service's SVG output.

This archive deployment alone does not complete M2. Light-client retrieval,
archive loading in the production browser UI, and the full wallet/gallery flow
remain part of the subsequent frontend integration.
