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
  const previousFetch = globalThis.fetch;
  globalThis.fetch = () => { throw new Error('Unexpected external asset fetch'); };
  try {
    // Only this one JS file is present: no wasm, bindings, or node_modules.
    const { createSeedsong } = await import(pathToFileURL(bundle).href);
    const music = await createSeedsong();
    assert.match(music.randomDna(), /^[a-f0-9]{64}$/);
    assert.equal(music, await createSeedsong());
  } finally {
    globalThis.fetch = previousFetch;
    await rm(directory, { recursive: true, force: true });
  }
});
