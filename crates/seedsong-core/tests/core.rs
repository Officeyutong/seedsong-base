//! Reuse the portable suites on native; wasm includes them in the library harness.
#![cfg(not(target_arch = "wasm32"))]

#[path = "suites/backend.rs"]
mod backend;
#[path = "suites/music.rs"]
mod music;
