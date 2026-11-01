import * as wasm from "../pkg/seedsong_core.js";
import bytes from "../pkg/seedsong_core_bg.wasm";
import { createLibrary } from "./api.js";
import type { Seedsong } from "./types.js";
export type * from "./types.js";

wasm.initSync({ module: bytes });
const library = createLibrary(wasm);
/** Return the shared library backed by the embedded WASM; no fetch is performed. */
export async function createSeedsong(): Promise<Seedsong> {
  return library;
}
