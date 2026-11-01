import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

test('the distributed JS file runs alone without external wasm assets or fetch', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'seedsong-js-bundle-'));
  const bundle = join(directory, 'music.mjs');
  await writeFile(bundle, await readFile(new URL('../dist/index.js', import.meta.url)));
  const loader = join(directory, 'loader.mjs');
  await writeFile(loader, await readFile(new URL('../dist/loader.js', import.meta.url)));
  const decoder = await readFile(new URL('../dist/decoder.wasm', import.meta.url));
  const previousFetch = globalThis.fetch;
  globalThis.fetch = () => { throw new Error('Unexpected external asset fetch'); };
  try {
    // Only this one JS file is present: no wasm, bindings, or node_modules.
    const { createSeedsong } = await import(pathToFileURL(bundle).href);
    const music = await createSeedsong();
    assert.match(music.randomDna(), /^[a-f0-9]{64}$/);
    assert.equal(music, await createSeedsong());
    const entry = await import(pathToFileURL(loader).href);
    assert.deepEqual(Object.keys(entry), ['loadSeedsong']);
    const dynamic = await entry.loadSeedsong(decoder);
    const dna = '00'.repeat(32);
    assert.deepEqual(dynamic.generateMusic({ dna }), music.generateMusic({ dna }));
  } finally {
    globalThis.fetch = previousFetch;
    await rm(directory, { recursive: true, force: true });
  }
});
