//! Single-threaded PCM rendering followed by WebAudio buffer playback.
//! No filesystem, subprocess, Rayon, atomics, or worker setup is required.
use super::MusicBackend;
use crate::{
    dna::Dna,
    music::Song,
    synth::{self, Options, Timbre},
};
use wasm_bindgen::prelude::*;
use web_sys::{AudioBuffer, AudioBufferSourceNode, AudioContext};

#[wasm_bindgen]
pub struct WebAudioBackend {
    context: AudioContext,
}
impl MusicBackend for WebAudioBackend {
    type Output = AudioBuffer;
    type Error = JsValue;
    fn render(&mut self, song: &Song, options: Options) -> Result<AudioBuffer, JsValue> {
        options.validate().map_err(js_error)?;
        let frames = synth::render(song, options);
        let buffer = self
            .context
            .create_buffer(2, frames.len() as u32, options.rate as f32)?;
        // Reuse one channel allocation instead of retaining two additional buffers.
        let mut channel = vec![0.; frames.len()];
        for i in 0..2 {
            for (sample, frame) in channel.iter_mut().zip(&frames) {
                *sample = frame[i];
            }
            buffer.copy_to_channel(&channel, i as i32)?;
        }
        Ok(buffer)
    }
}
fn js_error(error: impl std::fmt::Display) -> JsValue {
    js_sys::Error::new(&error.to_string()).into()
}
#[wasm_bindgen]
impl WebAudioBackend {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<WebAudioBackend, JsValue> {
        Ok(Self {
            context: AudioContext::new()?,
        })
    }
    /// Call from a user gesture and await this Promise before starting playback.
    pub fn resume(&self) -> Result<js_sys::Promise, JsValue> {
        self.context.resume()
    }
    /// Release browser audio resources when finished. Stop active sources first.
    pub fn close(&self) -> Result<js_sys::Promise, JsValue> {
        self.context.close()
    }
    /// Render once, then reuse the buffer for playback. Rendering is synchronous.
    #[wasm_bindgen(js_name = renderDna)]
    pub fn render_dna(
        &mut self,
        dna: &str,
        piano: bool,
        looping: bool,
    ) -> Result<AudioBuffer, JsValue> {
        let dna = Dna::parse(dna).map_err(js_error)?;
        let options = Options {
            rate: self.context.sample_rate() as u32,
            timbre: if piano { Timbre::Piano } else { Timbre::Spec },
            looping,
            ..Options::default()
        };
        super::generate(self, dna, [None; 4], options)
    }
    /// Returns a source that the caller can stop/disconnect. Each play creates a new source.
    pub fn play(
        &self,
        buffer: &AudioBuffer,
        looping: bool,
    ) -> Result<AudioBufferSourceNode, JsValue> {
        let source = self.context.create_buffer_source()?;
        source.set_buffer(Some(buffer));
        source.set_loop(looping);
        source.connect_with_audio_node(&self.context.destination())?;
        if let Err(error) = source.start() {
            let _ = source.disconnect();
            return Err(error);
        }
        Ok(source)
    }
}
/// Return canonical SNG1 events to JavaScript without creating an AudioContext.
#[wasm_bindgen(js_name = encodeDna)]
pub fn encode_dna(dna: &str) -> Result<Vec<u8>, JsValue> {
    Ok(Song::generate(Dna::parse(dna).map_err(js_error)?, [None; 4]).encode())
}
#[wasm_bindgen(js_name = randomDna)]
pub fn random_dna() -> String {
    Dna::random().to_string()
}

/// Render a downloadable WAV without an AudioContext or filesystem.
#[wasm_bindgen(js_name = renderWav)]
pub fn render_wav(dna: &str, piano: bool, looping: bool) -> Result<Vec<u8>, JsValue> {
    let song = Song::generate(Dna::parse(dna).map_err(js_error)?, [None; 4]);
    let options = Options {
        timbre: if piano { Timbre::Piano } else { Timbre::Spec },
        looping,
        ..Options::default()
    };
    super::PcmAudio {
        frames: synth::render(&song, options),
        rate: options.rate,
    }
    .to_wav()
    .map_err(js_error)
}
