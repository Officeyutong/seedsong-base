import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as entry from '../dist/index.js';

const [music, concurrent] = await Promise.all([
  entry.createSeedsong(), entry.createSeedsong(),
]);
const dna = '9a401c07e8553fa2710c0000000000000209b62d000000000000000000000000';

test('minimal public API and shared initialization', () => {
  assert.deepEqual(Object.keys(entry), ['createSeedsong']);
  assert.deepEqual(Object.keys(music).sort(), ['randomDna', 'renderAudioBuffer', 'renderWav']);
  assert.equal(music, concurrent);
  assert.ok(Object.isFrozen(music));
  assert.match(music.randomDna(), /^[a-f0-9]{64}$/);
});
test('structured serde input and typed WAV result, no browser required', () => {
  const result = music.renderWav({ dna, sampleRate: 22050, loop: true, reverb: 0, timbre: 'piano' });
  assert.ok(result.data instanceof Uint8Array);
  assert.equal(result.dna, dna);
  assert.equal(result.musicVersion, 2);
  assert.equal(result.sampleRate, 22050);
  assert.equal(result.channels, 2);
  const view = new DataView(result.data.buffer, result.data.byteOffset, result.data.byteLength);
  assert.equal(new TextDecoder().decode(result.data.slice(0, 4)), 'RIFF');
  assert.equal(view.getUint32(24, true), result.sampleRate);
  assert.equal(view.getUint32(40, true) + 44, result.data.length);
  const frames = Math.round(256 * 60 / 124 / 4 * result.sampleRate);
  assert.equal(result.data.length, 44 + frames * 4);
  assert.equal(result.durationSeconds, frames / result.sampleRate);
});
test('hex and Uint8Array DNA produce the same sound and canonical metadata', () => {
  const raw = Uint8Array.from(Buffer.from(dna, 'hex'));
  const a = music.renderWav({ dna: raw, sampleRate: 22050 });
  const b = music.renderWav({ dna: `0x${dna.toUpperCase()}`, sampleRate: 22050, timbre: undefined, loop: undefined, reverb: undefined });
  assert.equal(a.dna, dna);
  assert.deepEqual(a.data, b.data);
  const saved = a.data.slice();
  music.renderWav({ dna: '00'.repeat(32), sampleRate: 22050 });
  assert.deepEqual(a.data, saved, 'returned bytes must survive wasm memory growth');
});
test('rejects ambiguous types, misspelled fields and invalid render values', () => {
  for (const options of [
    null, {}, { dna: [] }, { dna: new Uint8Array(31) }, { dna: 'invalid' },
    { dna, timbre: true }, { dna, timbre: 'flute' }, { dna, loop: 'true' },
    { dna, sampleRate: 22050.5 }, { dna, sampleRate: 0 }, { dna, sampleRate: null },
    { dna, sample_rate: 44100 }, { dna, reverb: NaN }, { dna, reverb: 2 },
    { dna, participants: null }, { dna, participants: { unknown: dna } },
    { dna, musicVersion: 0 }, { dna, musicVersion: 3 }, { dna, musicVersion: 1.5 }, { dna, musicVersion: '2' },
  ]) assert.throws(() => music.renderWav(options), Error, JSON.stringify(options));
});
test('music versions are explicit and legacy output remains available', () => {
  const options = { dna, sampleRate: 22050, loop: true, reverb: 0 };
  const legacy = music.renderWav({ ...options, musicVersion: 1 });
  const current = music.renderWav({ ...options, musicVersion: 2 });
  assert.equal(legacy.musicVersion, 1);
  assert.equal(current.musicVersion, 2);
  assert.equal(legacy.durationSeconds, current.durationSeconds);
  assert.notDeepEqual(legacy.data, current.data);
});
test('ensemble role validation reaches Rust through serde', () => {
  const conductor = new Uint8Array(32); // melody role
  const bass = new Uint8Array(32); bass[16] = 1;
  const result = music.renderWav({ dna: conductor, sampleRate: 22050, participants: { bass } });
  assert.equal(result.dna, '00'.repeat(32));
  assert.throws(() => music.renderWav({ dna: conductor, participants: { melody: conductor } }), /conductor/);
  assert.throws(() => music.renderWav({ dna: conductor, participants: { bass: conductor } }), /role/);
});
test('AudioBuffer method requires a browser context', () => {
  assert.throws(() => music.renderAudioBuffer(undefined, { dna }), /AudioContext/);
});
