import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { createSeedsong } from '../dist/index.js';

const root = fileURLToPath(new URL('../../../', import.meta.url));
const readJson = path => JSON.parse(readFileSync(new URL(path, import.meta.url), 'utf8'));
const vectors = readJson('../../../vectors/dob-parity.json');
assert.equal(vectors.schema, 'seedsong-dob-parity/1');
const music = await createSeedsong();
let executable;

// Compile once per test process. Cargo caches the commit-pinned upstream libs.
// Resolve the artifact via Cargo rather than assuming a target directory or OS.
function official(request) {
  if (!executable) {
    const build = spawnSync('cargo', [
      'build', '--locked', '-p', 'seedsong-core', '--example', 'dob-reference',
      '--message-format=json-render-diagnostics',
    ], { cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024, timeout: 300_000 });
    assert.ifError(build.error);
    assert.equal(build.status, 0, `Cannot build official reference adapter:\n${build.stderr}\n${build.stdout}`);
    for (const line of build.stdout.trim().split('\n')) {
      const message = JSON.parse(line);
      if (message.reason === 'compiler-artifact' && message.target.name === 'dob-reference' && message.executable) executable = message.executable;
    }
    assert.ok(executable, 'Cargo did not produce the native reference adapter');
  }
  const result = spawnSync(executable, [], {
    input: JSON.stringify(request), encoding: 'utf8', cwd: root,
    maxBuffer: 128 * 1024 * 1024, timeout: 120_000,
  });
  assert.ifError(result.error);
  assert.equal(result.status, 0, `Official decoder failed:\n${result.stderr}`);
  const lines = result.stdout.trim().split('\n').map(line => JSON.parse(line));
  assert.equal(lines.length, request.dnas.length, 'Missing official decoder results');
  return lines;
}

function compare({ dnas, dob0, dob1 }) {
  const expected = official({ dnas, dob0, dob1 });
  const svgBytes = outputs => outputs.flatMap(entry => entry.traits)
    .filter(trait => Object.hasOwn(trait, 'SVG')).map(trait => Buffer.from(trait.SVG, 'utf8'));
  let svgCount = 0;
  for (let index = 0; index < dnas.length; index++) {
    const dna = dnas[index];
    const reference = expected[index];
    assert.equal(reference.dna, dna, 'Reference adapter reordered DNA');
    const traits = music.decodeDob0({ dna, pattern: dob0 });
    assert.deepEqual(traits, reference.dob0, `DOB/0 DNA=${dna}`);
    // Each implementation receives its OWN DOB/0 output: exercise both stages.
    const result = music.decodeDob1({ traits, pattern: dob1 });
    assert.deepEqual(result, reference.dob1, `DOB/1 DNA=${dna}`);
    const actual = svgBytes(result);
    assert.deepEqual(actual, svgBytes(reference.dob1), `SVG UTF-8 bytes DNA=${dna}`);
    svgCount += actual.length;
  }
  return svgCount;
}

test('WASM equals official DOB/0 + DOB/1 for 1,000 collection DNA and byte boundaries', t => {
  const document = readFileSync(new URL('../../../pattern.md', import.meta.url), 'utf8');
  const patterns = [...document.matchAll(/```json\n([\s\S]*?)\n```/g)]
    .map(match => JSON.parse(match[1])).filter(Array.isArray);
  assert.equal(patterns.length, 2);
  const report = readJson('../../../reports/m1-v2/quality-report.json');
  assert.equal(report.songs.length, 1000);
  const reference = '9a401c07e8553fa2710c0000000000000209b62d000000000000000000000000';
  const dnas = [reference, '00'.repeat(32), 'ff'.repeat(32), ...report.songs.map(song => song.dna)];
  for (let offset = 0; offset < 32; offset++) {
    for (const value of [0, 1, 254, 255]) {
      const dna = Buffer.from(reference, 'hex'); dna[offset] = value;
      dnas.push(dna.toString('hex'));
    }
  }
  const svgCount = compare({ dnas, dob0: patterns[0], dob1: patterns[1] });
  assert.equal(svgCount, dnas.length, 'Every collection DNA must produce one cover');
  t.diagnostic(`Compared ${dnas.length} DNA and ${svgCount} SVG covers byte-for-byte against pinned official decoders.`);
});

for (const vector of vectors.cases) {
  test(`WASM official DOB semantics: ${vector.name}`, () => compare(vector));
}
