//! A backend converts deterministic events into platform-specific audio.
use crate::{dna::Dna, music::Song, synth::Options};

#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(target_arch = "wasm32")]
pub mod web_audio;

/// No Send/Sync requirement: browser audio objects belong to their JS thread.
pub trait MusicBackend {
    type Output;
    type Error;
    fn render(&mut self, song: &Song, options: Options) -> Result<Self::Output, Self::Error>;
}

/// Decode a DNA (and optional ensemble) and render it with a chosen backend.
pub fn generate<B: MusicBackend>(
    backend: &mut B,
    conductor: Dna,
    participants: [Option<Dna>; 4],
    options: Options,
) -> Result<B::Output, B::Error> {
    backend.render(&Song::generate(conductor, participants), options)
}

/// Interleaved stereo samples in [-1, 1], with their sample rate.
#[derive(Debug)]
pub struct PcmAudio {
    pub frames: Vec<[f32; 2]>,
    pub rate: u32,
}

impl PcmAudio {
    /// Encode a stereo PCM16 WAV in memory, including on wasm (download as a Blob).
    pub fn to_wav(&self) -> anyhow::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        crate::wav::write(&mut bytes, &self.frames, self.rate)?;
        Ok(bytes)
    }
}
