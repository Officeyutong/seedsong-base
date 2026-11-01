//! DNA decoding, PCM synthesis, and platform-specific music backends.
macro_rules! platform_iter {
    ($parallel:expr, $serial:expr) => {{
        #[cfg(not(target_arch = "wasm32"))]
        {
            $parallel
        }
        #[cfg(target_arch = "wasm32")]
        {
            $serial
        }
    }};
}
pub mod backend;
pub mod dna;
pub mod dob;
pub mod music;
pub mod rng;
pub mod synth;
pub use backend::{MusicBackend, generate};

mod wav;

// One wasm test binary contains both unit and integration-style suites.
// Interactive browser runs show every test on the same page.
#[cfg(all(test, target_arch = "wasm32"))]
#[path = "../tests/support/mod.rs"]
mod wasm_test_config;

#[cfg(all(test, target_arch = "wasm32"))]
extern crate self as seedsong_core;

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "../tests/suites/backend.rs"]
mod backend_tests;
#[cfg(all(test, target_arch = "wasm32"))]
#[path = "../tests/suites/music.rs"]
mod music_tests;
#[cfg(all(test, target_arch = "wasm32"))]
#[path = "../tests/suites/wasm.rs"]
mod wasm_tests;
#[cfg(all(test, target_arch = "wasm32"))]
#[path = "../tests/suites/web_audio.rs"]
mod web_audio_tests;

#[cfg(target_arch = "wasm32")]
mod js;

// Independent official decoders are native-only development dependencies.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod dob_official_tests;
