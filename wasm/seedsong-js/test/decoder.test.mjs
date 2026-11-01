import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { createSeedsong } from '../dist/index.js';
import { loadSeedsong } from '../dist/loader.js';

const music = await createSeedsong();
const dna = '9a401c07e8553fa2710c0000000000000209b62d000000000000000000000000';
const document = await readFile(new URL('../../../pattern.md', import.meta.url), 'utf8');
const [dob0, dob1] = [...document.matchAll(/```json\n([\s\S]*?)\n```/g)]
  .map(match => JSON.parse(match[1])).filter(Array.isArray);

test('Rust DOB decoding consumes the real collection patterns and returns standard objects', () => {
  const traits = music.decodeDob0({ dna, pattern: dob0 });
  assert.deepEqual(traits.find(t => t.name === 'Tempo'), { name: 'Tempo', traits: [{ Number: 124 }] });
  const result = music.decodeDob1({ traits, pattern: dob1 });
  assert.deepEqual(result.slice(0, traits.length), traits);
  const svg = result.at(-1).traits[0].SVG;
  assert.ok(svg.startsWith('<svg '));
  assert.ok(svg.endsWith('</svg>'));
  assert.ok(svg.includes("color='#F4D35E'"));
  assert.equal(svg.includes('{value}'), false);
});

test('DOB supports arbitrary DNA length, 64-bit native segments, typed ranges and hidden traits', () => {
  const traits = music.decodeDob0({ dna: new Uint8Array([255, 1, 65, 0]), pattern: [
    ['n', 'Number', 0, 2, 'rawNumber'],
    ['_text', 'String', 2, 100, 'utf8'],
    ['r', 'Number', 0, 1, 'range', [60, 141]],
  ] });
  assert.deepEqual(traits, [
    { name: 'n', traits: [{ Number: 511 }] },
    { name: '_text', traits: [{ String: 'A' }] },
    { name: 'r', traits: [{ Number: 72 }] },
  ]);
  const result = music.decodeDob1({ traits, pattern: [
    ['IMAGE.0', 'elements', '_text', 'raw', '<text>{value}</text>'],
    ['IMAGE.0', 'elements', 'r', 'range', [[[60, 72], '<a/>'], [['*'], '<b/>']]],
  ] });
  assert.deepEqual(result, [traits[0], traits[2], { name: 'IMAGE', traits: [{ SVG: '<svg><text>A</text><a/></svg>' }] }]);
  assert.throws(() => music.decodeDob0({ dna: 'ff'.repeat(8), pattern: [['n', 'Number', 0, 8, 'rawNumber']] }), /JavaScript number/);
  for (const options of [null, { dna, pattern: [['bad']] }, { dna, pattern: [], typo: true }, { dna: '', pattern: [] }, { dna, pattern: [['n', 'Number', 0, 1, 'options', []]] }]) {
    assert.throws(() => music.decodeDob0(options));
  }
  assert.throws(() => music.decodeDob1({ traits, pattern: [], typo: true }));
});

test('generated music matches every existing native SNG1 vector for both versions', async () => {
  for (const version of [1, 2]) {
    const { vectors } = JSON.parse(await readFile(new URL(`../../../vectors/music-v${version}.json`, import.meta.url), 'utf8'));
    for (const vector of vectors) {
      const result = music.generateMusic({ dna: vector.dna, musicVersion: version });
      assert.equal(Buffer.from(result.sng1).toString('hex'), vector.sng1_hex);
      assert.equal(result.params.bpm, result.sng1[5]);
      assert.equal(result.events.length, new DataView(result.sng1.buffer).getUint16(16, true));
      assert.equal(result.events[0].start, new DataView(result.sng1.buffer).getUint16(18, true));
    }
  }
  assert.throws(() => music.generateMusic({ dna, sampleRate: 22050 }), /Unknown option/);
});

test('Cell bytes load without fetch; instances and the embedded default remain independent', async () => {
  const bytes = await readFile(new URL('../dist/decoder.wasm', import.meta.url));
  const manifest = JSON.parse(await readFile(new URL('../dist/decoder.json', import.meta.url), 'utf8'));
  assert.equal(createHash('sha256').update(bytes).digest('hex'), manifest.sha256);
  assert.equal(bytes.length, manifest.wasmBytes);
  const previousFetch = globalThis.fetch;
  globalThis.fetch = () => { throw new Error('Unexpected network request'); };
  try {
    // A slice tests non-zero byteOffset handling, as returned by Cell codecs.
    const padded = new Uint8Array(bytes.length + 8); padded.set(bytes, 4);
    const a = await loadSeedsong(padded.subarray(4, bytes.length + 4));
    const b = await loadSeedsong(await WebAssembly.compile(bytes));
    assert.notEqual(a, b);
    const expected = music.generateMusic({ dna });
    assert.deepEqual(a.generateMusic({ dna }), expected);
    b.generateMusic({ dna: '00'.repeat(32) });
    assert.deepEqual(a.generateMusic({ dna }), expected);
    assert.deepEqual(music.generateMusic({ dna }), expected);
    assert.deepEqual(a.decodeDob0({ dna, pattern: dob0 }), music.decodeDob0({ dna, pattern: dob0 }));
    const options = { dna, sampleRate: 22050, loop: true, reverb: 0 };
    assert.deepEqual(a.renderWav(options), music.renderWav(options));
    await assert.rejects(loadSeedsong('https://example.com/decoder.wasm'), TypeError);
    await assert.rejects(loadSeedsong(new Uint8Array([0, 1, 2])), WebAssembly.CompileError);
    await assert.rejects(loadSeedsong(new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0])), /Incompatible/);
  } finally { globalThis.fetch = previousFetch; }
});
