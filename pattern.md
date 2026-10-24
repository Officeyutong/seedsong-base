# Seedsong DOB Patterns v1

**Pattern identifier:** `seedsong-pattern/1`

**Music specification:** [`seedsong-music/2`](spec.md); legacy [`seedsong-music/1`](spec-v1.md)

This document fixes the DOB/0 traits and DOB/1 cover pattern for Seedsong's 32-byte DNA. It completes the cover byte allocation reserved in the music specification. The cover patterns and traits are unchanged by music v2. The music reference must name the version implemented by the selected WASM artifact. These are collection design decisions; the JSON patterns below are normative. This document specifies the patterns and their packaging, not a deployed Cluster or an implemented browser cover renderer.

The [Seedsong proposal](https://talk.nervos.org/t/spark-program-seedsspark-program-seedsong-on-chain-generative-music-with-spore-dob-and-light-client-wasmong-spore-dob-light-client-wasm/10997) calls for standard DOB decoders for wallet-visible traits and layered covers, plus a separate music WASM referenced from the Cluster. A browser retrieves the necessary Cells through light-client-wasm and restores the artwork and music locally. Audio files are generated rather than stored on-chain.

## 1. Decoder composition

Use one DOB/1 Cluster with this ordered decoding pipeline:

```text
32-byte DNA -> official DOB/0 decoder -> musical and cover traits
                                    -> official DOB/1 decoder -> SVG cover

32-byte DNA -> Seedsong music WASM -> deterministic SNG1 events -> audio
```

DOB/0 is the first stage of the DOB/1 collection, not a second collection to mint. The music WASM is referenced by a Seedsong extension outside `dob.decoders`; it does not participate in the standard SVG decoder chain. The [official DOB/1 implementation](https://github.com/sporeprotocol/spore-dob-1/blob/master/src/decoder/mod.rs) preserves the DOB/0 traits and appends an `IMAGE` entry with an `SVG` trait.

Use `contentType: "dob/1"` for Seedsong records. The canonical Spore content is the UTF-8 encoding of a JSON object with one `dna` field: exactly 64 lowercase hexadecimal digits, without a prefix or separators. For example:

```json
{"dna":"9a401c07e8553fa2710c0000000000000209b62d000000000000000000000000"}
```

Set the Spore's `clusterId` to the Cluster containing the patterns below. The content is JSON bytes containing a hex string, not the raw 32 bytes or the UTF-8 bytes of an unquoted hex string. User input may accept the broader formats supported by the Rust library; normalize before minting. Reject decoded DNA lengths other than 32 bytes before rendering or minting.

## 2. Selection rules and byte allocation

All offsets are zero-based. Each DOB/0 entry below reads exactly one byte.

- `options`: choose `args[D[offset] % args.length]`. Repeated entries provide weights.
- DOB/0 `range`: choose `lower + D[offset] % (upper - lower)`. The upper bound is excluded, so `[60, 141]` produces 60–140 BPM.
- DOB/1 numeric conditions such as `[60, 79]` include both endpoints. They match the already decoded BPM, not the original DNA byte.

These rules follow the decoder source: [DOB/0 selection](https://github.com/sporeprotocol/spore-dob-0/blob/master/src/decoder/mod.rs) and [DOB/1 matching](https://github.com/sporeprotocol/spore-dob-1/blob/master/src/decoder/mod.rs). In particular, do not interpret the DOB/0 range as inclusive just because a prose example uses interval notation.

| Byte(s) | Meaning | DOB/0 trait | Cover use |
| --- | --- | --- | --- |
| 0 | Weighted musical mode | `Mode` | Foreground color |
| 1 | Tempo, 60–140 BPM | `Tempo` | Rounded, mixed, or angular aura |
| 2 | Chord progression | Not exposed | None |
| 3 | Weighted melodic contour | `Contour` | None |
| 4 | Weighted lead timbre | `Lead` | None |
| 5 | Weighted bass timbre | `Bass` | None |
| 6–9 | Melody seed | Not exposed | None |
| 10 | Background palette | `Palette` | Full-canvas background |
| 11 | Main silhouette | `Shape` | Central shape |
| 12 | Background texture | `Texture` | Texture beneath the main shape |
| 13 | Decorative accents | `Accent` | Foreground details |
| 14 | Main-layer orientation | `Orientation` | Rotate the shape and tempo aura together |
| 15 | Border style | `Frame` | Topmost border |
| 16 | Ensemble role | `Role` | None |
| 17 | Musical key | `Key` | None |
| 18 | Arrangement bitfields | `Form` only | None |
| 19 | Groove bitfields | Not exposed | None |
| 20–31 | Variation and noise seeds | Not exposed | None |

The six cover bytes use four equally weighted choices each. Each trait value therefore occurs for 64 of the 256 possible byte values. Different DNA values can share traits and artwork; artwork is not a unique identifier. Musical weights and names remain those in `spec.md`. Tempo and key have the modulo bias already implied by that specification.

## 3. DOB/0 pattern

The following array is the complete value of the first decoder's `pattern`. Keep the repeated entries and their order exactly as written. Trait names and string values are case-sensitive.

```json
[
  ["Mode", "String", 0, 1, "options", [
    "Major", "Major", "Major", "Major", "Major", "Major", "Major", "Major",
    "Minor", "Minor", "Minor", "Minor", "Minor", "Minor", "Minor", "Minor",
    "Dorian", "Dorian", "Dorian", "Dorian",
    "Mixolydian", "Mixolydian", "Mixolydian", "Mixolydian",
    "Major Pentatonic", "Major Pentatonic", "Major Pentatonic", "Major Pentatonic",
    "Minor Pentatonic", "Minor Pentatonic", "Minor Pentatonic", "Lydian"
  ]],
  ["Tempo", "Number", 1, 1, "range", [60, 141]],
  ["Contour", "String", 3, 1, "options", [
    "Smooth", "Smooth", "Smooth", "Balanced", "Balanced", "Balanced", "Balanced", "Leapy"
  ]],
  ["Lead", "String", 4, 1, "options", [
    "Soft Sine", "Soft Sine", "Soft Sine", "Soft Sine", "Soft Sine",
    "Soft Sine", "Soft Sine", "Soft Sine", "Soft Sine", "Soft Sine",
    "Warm Organ", "Warm Organ", "Warm Organ", "Warm Organ", "Warm Organ", "Warm Organ", "Warm Organ",
    "E-Piano", "E-Piano", "E-Piano", "E-Piano", "E-Piano", "E-Piano",
    "Glass Bell", "Glass Bell", "Glass Bell", "Glass Bell", "Glass Bell",
    "Music Box", "Music Box", "Music Box", "Crystal"
  ]],
  ["Bass", "String", 5, 1, "options", ["Sine Sub", "Sine Sub", "Round FM", "Pluck Bass"]],
  ["Palette", "String", 10, 1, "options", ["Midnight", "Slate", "Plum", "Forest"]],
  ["Shape", "String", 11, 1, "options", ["Orb", "Diamond", "Wave", "Seed"]],
  ["Texture", "String", 12, 1, "options", ["Grid", "Waves", "Rays", "Dots"]],
  ["Accent", "String", 13, 1, "options", ["None", "Stars", "Satellites", "Orbit"]],
  ["Orientation", "String", 14, 1, "options", ["North", "East", "South", "West"]],
  ["Frame", "String", 15, 1, "options", ["None", "Single", "Double", "Corners"]],
  ["Role", "String", 16, 1, "options", ["Melody", "Bass", "Harmony", "Rhythm"]],
  ["Key", "String", 17, 1, "options", ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"]],
  ["Form", "String", 18, 1, "options", ["AABA", "ABAB", "AABB", "ABAC"]]
]
```

`Role` describes the record's ensemble role; an individual record still generates the full arrangement. `Harmony` corresponds to the JS ensemble part `pad`, and `Rhythm` to `drums`. The displayed `Lead` is the DNA-selected timbre, even if a listener chooses the optional piano rendering override. Such playback settings do not alter collectible traits.

Do not expose the complete arrangement byte as a Form index: `% 4` selects Form, while its upper bits select other patterns. Groove and seeds remain music-only parameters. Cover generation does not consume any music random stream.

## 4. DOB/1 cover pattern

The artwork is a static, self-contained 512 × 512 SVG. Every path and color is embedded in the pattern. No image URI, font, script, animation, remote resource, or generated raster asset is required.

The layer order is background, texture, rotated tempo aura and main shape, accents, then frame. Mode sets the root `color`; `currentColor` carries it into the shape and decorations. Minor-family modes use blue, teal, or violet. The palette selects a dark background independently. The decoded tempo selects rounded geometry at 60–79 BPM, mixed geometry at 80–109, and angular geometry at 110–140.

All entries target `IMAGE.0` and must remain contiguous and in the order below. The official decoder gathers attributes into the root SVG and concatenates elements without inserting separators. The separate opening and closing group fragments below intentionally surround the aura and shape; individual fragments need not be standalone SVG documents.

The following array is the complete value of the second decoder's `pattern`:

```json
[
  ["IMAGE.0", "attributes", "", "raw", "xmlns='http://www.w3.org/2000/svg' viewBox='0 0 512 512' width='512' height='512'"],
  ["IMAGE.0", "attributes", "Mode", "options", [
    ["Major", "color='#FFBF69'"],
    ["Minor", "color='#7AA2F7'"],
    ["Dorian", "color='#65D6C5'"],
    ["Mixolydian", "color='#FF9866'"],
    ["Major Pentatonic", "color='#F4D35E'"],
    ["Minor Pentatonic", "color='#B39DDB'"],
    ["Lydian", "color='#FF9ED2'"]
  ]],
  ["IMAGE.0", "elements", "Palette", "options", [
    ["Midnight", "<rect width='512' height='512' fill='#0B1026'/>"],
    ["Slate", "<rect width='512' height='512' fill='#14232E'/>"],
    ["Plum", "<rect width='512' height='512' fill='#27162E'/>"],
    ["Forest", "<rect width='512' height='512' fill='#102A25'/>"]
  ]],
  ["IMAGE.0", "elements", "Texture", "options", [
    ["Grid", "<path d='M64 0V512M128 0V512M192 0V512M256 0V512M320 0V512M384 0V512M448 0V512M0 64H512M0 128H512M0 192H512M0 256H512M0 320H512M0 384H512M0 448H512' fill='none' stroke='#FFFFFF' stroke-opacity='.08'/>"] ,
    ["Waves", "<path d='M0 96Q128 32 256 96T512 96M0 192Q128 128 256 192T512 192M0 288Q128 224 256 288T512 288M0 384Q128 320 256 384T512 384' fill='none' stroke='#FFFFFF' stroke-opacity='.1' stroke-width='2'/>"] ,
    ["Rays", "<path d='M256 256L0 0M256 256L256 0M256 256L512 0M256 256L512 256M256 256L512 512M256 256L256 512M256 256L0 512M256 256L0 256' fill='none' stroke='#FFFFFF' stroke-opacity='.1' stroke-width='2'/>"] ,
    ["Dots", "<g fill='#FFFFFF' fill-opacity='.15'><circle cx='96' cy='96' r='3'/><circle cx='256' cy='96' r='3'/><circle cx='416' cy='96' r='3'/><circle cx='96' cy='256' r='3'/><circle cx='416' cy='256' r='3'/><circle cx='96' cy='416' r='3'/><circle cx='256' cy='416' r='3'/><circle cx='416' cy='416' r='3'/></g>"]
  ]],
  ["IMAGE.0", "elements", "Orientation", "options", [
    ["North", "<g transform='rotate(0 256 256)'>"],
    ["East", "<g transform='rotate(90 256 256)'>"],
    ["South", "<g transform='rotate(180 256 256)'>"],
    ["West", "<g transform='rotate(270 256 256)'>"]
  ]],
  ["IMAGE.0", "elements", "Tempo", "options", [
    [[60, 79], "<g fill='none' stroke='currentColor' stroke-opacity='.35' stroke-width='3'><circle cx='256' cy='256' r='164'/><circle cx='256' cy='256' r='184'/></g>"],
    [[80, 109], "<g fill='none' stroke='currentColor' stroke-opacity='.35' stroke-width='3'><circle cx='256' cy='256' r='174'/><path d='M256 64L448 256L256 448L64 256Z'/></g>"],
    [[110, 140], "<path d='M256 56L456 256L256 456L56 256ZM256 80L432 256L256 432L80 256Z' fill='none' stroke='currentColor' stroke-opacity='.35' stroke-width='3'/>"]
  ]],
  ["IMAGE.0", "elements", "Shape", "options", [
    ["Orb", "<circle cx='256' cy='256' r='116' fill='currentColor'/><circle cx='224' cy='224' r='32' fill='#FFFFFF' fill-opacity='.2'/>"],
    ["Diamond", "<path d='M256 116L396 256L256 396L116 256Z' fill='currentColor'/><path d='M256 148L364 256H256Z' fill='#FFFFFF' fill-opacity='.2'/>"],
    ["Wave", "<path d='M116 256C156 116 216 116 256 256S356 396 396 256' fill='none' stroke='currentColor' stroke-width='40' stroke-linecap='round'/>"],
    ["Seed", "<path d='M256 112C416 208 416 336 256 400C96 336 96 208 256 112Z' fill='currentColor'/><path d='M256 152V360' fill='none' stroke='#FFFFFF' stroke-opacity='.25' stroke-width='4'/>"]
  ]],
  ["IMAGE.0", "elements", "", "raw", "</g>"],
  ["IMAGE.0", "elements", "Accent", "options", [
    ["None", ""],
    ["Stars", "<path d='M96 112V144M80 128H112M416 368V400M400 384H432' fill='none' stroke='#FFFFFF' stroke-width='4' stroke-linecap='round'/>"],
    ["Satellites", "<g fill='currentColor'><circle cx='104' cy='176' r='12'/><circle cx='408' cy='336' r='8'/><circle cx='368' cy='104' r='5'/></g>"],
    ["Orbit", "<ellipse cx='256' cy='256' rx='208' ry='76' transform='rotate(-25 256 256)' fill='none' stroke='#FFFFFF' stroke-opacity='.55' stroke-width='3'/>"]
  ]],
  ["IMAGE.0", "elements", "Frame", "options", [
    ["None", ""],
    ["Single", "<rect x='24' y='24' width='464' height='464' rx='16' fill='none' stroke='#FFFFFF' stroke-opacity='.4' stroke-width='2'/>"] ,
    ["Double", "<g fill='none' stroke='#FFFFFF' stroke-opacity='.4' stroke-width='2'><rect x='24' y='24' width='464' height='464' rx='16'/><rect x='32' y='32' width='448' height='448' rx='12'/></g>"] ,
    ["Corners", "<path d='M24 96V24H96M416 24H488V96M488 416V488H416M96 488H24V416' fill='none' stroke='#FFFFFF' stroke-opacity='.6' stroke-width='3'/>"]
  ]]
]
```

There are no fallback branches: every value produced by the DOB/0 pattern is explicitly covered. A Seedsong renderer must reject missing or invalid required traits instead of quietly producing a partial cover. The unmodified official decoder may skip unmatched branches, so validate the input and pattern before invoking it.

The canonical cover is the exact SVG string assembled by the official decoder. Preserve fragment spelling, whitespace, attribute order, and entry order for byte comparison. A local renderer may display that string directly; XML reserialization or optimization must not replace it in conformance fixtures. Pixel output can still vary by SVG renderer.

## 5. Cluster description and music reference

Use the modern `description.dob.ver = 1` envelope with two decoder entries. The `description` field of Cluster data is a **JSON string**. Inside that string, each `pattern` is a JSON array, not a second string containing JSON. This matches [CCC's encoder](https://github.com/ckb-devrel/ccc/blob/master/packages/spore/src/dob/api/encode.ts) and [DOB/1 example](https://github.com/sporeprotocol/dob-cookbook/blob/main/examples/dob1/0.basic-shape.ts).

The following JavaScript constructs the string from the two arrays above, assigned to `dob0Pattern` and `dob1Pattern` respectively:

```javascript
function makeClusterDescription(dob0Pattern, dob1Pattern, musicDecoderHash) {
  if (!/^0x[0-9a-f]{64}$/.test(musicDecoderHash)) {
    throw new Error('Expected the deployed music WASM Cell data hash');
  }
  return JSON.stringify({
    description: 'Seedsong: deterministic music and SVG covers from 32-byte DNA.',
    dob: {
      ver: 1,
      decoders: [
        {
          decoder: {
            type: 'code_hash',
            hash: '0x13cac78ad8482202f18f9df4ea707611c35f994375fa03ae79121312dda9925c',
          },
          pattern: dob0Pattern,
        },
        {
          decoder: {
            type: 'code_hash',
            hash: '0xda3525549b72970b4c95f5b5749357f20d1293d335710b674f09c32f7d54b6dc',
          },
          pattern: dob1Pattern,
        },
      ],
    },
    seedsong: {
      pattern: 'seedsong-pattern/1',
      music: {
        spec: 'seedsong-music/2',
        decoder: { type: 'code_hash', hash: musicDecoderHash },
      },
    },
  });
}
```

The official decoder hashes above are listed in [CCC's network configuration](https://github.com/ckb-devrel/ccc/blob/master/packages/spore/src/dob/config/predefined.ts) for both mainnet and testnet. `code_hash` here means the CKB hash of the decoder Cell's data. Resolve the corresponding Cell on the selected network and verify its data hash before execution; it is not the hash of the Cell's lock or type script. Network-specific transaction locations must be resolved separately.

Use immutable `code_hash` references for v1. The proposal also permits Type ID references, but those can resolve to updated code. A release using a mutable reference would need an additional immutable expected data hash to preserve reproducibility.

`seedsong` is a project extension, not a new DOB protocol version or a standard wallet audio field. Generic DOB clients only need `dob` to recover the cover and traits. Seedsong-aware clients additionally resolve `seedsong.music.decoder` and execute music locally. Do not invent an `AUDIO` trait or append music WASM as a third standard decoder.

The music hash must come from an actual deployed artifact; this document intentionally supplies no placeholder deployment hash. The current JS library bundles WASM and its wasm-bindgen initialization code. Fetching a WASM Cell alone does not make it compatible with that library: an on-chain-loading release must also provide matching wasm-bindgen glue or define and implement a standalone ABI. This document fixes discovery metadata without claiming that loader or deployment already exists.

If using CCC's `encodeClusterDescriptionForDob1`, convert each tuple into its corresponding `PatternElementDob0` or `PatternElementDob1` object first. That helper installs tuple serialization on object-form inputs; do not pass these already serialized tuples to it. Direct `JSON.stringify` as above preserves the required tuple format and the Seedsong extension.

## 6. Reference DNA

For the DNA shown in Section 1, DOB/0 must return these values in the pattern's order:

| Trait | Value |
| --- | --- |
| Mode | Major Pentatonic |
| Tempo | 124 (Number) |
| Contour | Leapy |
| Lead | Soft Sine |
| Bass | Sine Sub |
| Palette | Midnight |
| Shape | Orb |
| Texture | Grid |
| Accent | None |
| Orientation | North |
| Frame | None |
| Role | Harmony |
| Key | A |
| Form | AABB |

The cover uses a `#0B1026` background, grid texture, `#F4D35E` foreground, the fast angular aura, and an unrotated Orb. Accents and frame contribute empty strings. DOB/1 appends an entry with `name: "IMAGE"`, whose first trait has `type: "SVG"` and the assembled SVG as its value. The music remains exactly the output prescribed by `spec.md`.

## 7. Remix inheritance

The six cover bytes are six independent structural inheritance groups: Palette `[10]`, Shape `[11]`, Texture `[12]`, Accent `[13]`, Orientation `[14]`, and Frame `[15]`. Inherit each selected parent's **whole byte**, not the decoded option index. Group selection and parent provenance are checked off-chain under the remix policy in `spec.md`; these patterns do not enforce ancestry on-chain.

Musical structural groups remain unchanged. Generate melody seed bytes 6–9 and variation/noise bytes 20–31 anew. Recompute all traits and SVG from the child's final DNA. Foreground color follows the child's Mode, and the aura follows the child's Tempo, even when its cover bytes came from another parent. There is no separately inherited color or tempo-aura byte.

## 8. Conformance and release requirements

Before publishing a Cluster using `seedsong-pattern/1`:

1. Check all 256 byte values for every exposed musical trait against the Rust decoder. Check repeated-list weights, BPM boundaries, keys, and Form independently of the upper arrangement bits.
2. Check all four choices of every cover selector, all seven modes, and BPM boundaries 60, 79, 80, 109, 110, and 140. Every combination must produce well-formed SVG with exactly one root and balanced groups.
3. Maintain at least 100 fixed DNA fixtures. Execute the pinned official DOB/0 and DOB/1 decoders and compare their complete decoded output, including the exact SVG string, against the local implementation. Use the reference DNA above as one fixture. A development decoder service may supply comparison results; production restoration must work locally from chain data.
4. Run browser checks for the SVG and music, then verify a minted sample in the target wallets and marketplaces. Standard decoder use does not by itself prove every viewer supports a specific Cluster or its custom music extension.
5. Record network, Cluster ID, decoder data hashes, source revisions, deployed music artifact, and fixture results in the release manifest. Confirm that the music artifact and its loader agree on their ABI.

These are acceptance requirements, not a claim that decoder parity, minting, or wallet compatibility has already been tested in this repository. Existing music regression vectors only cover the music implementation.

Any change to trait names, option order or repetition, offsets, SVG fragments, layer order, or interpretation requires a new pattern identifier and a new Cluster release. Existing records retain their original Cluster's interpretation. A cover-only release need not change `seedsong-music/1` when the music algorithm is unchanged.
