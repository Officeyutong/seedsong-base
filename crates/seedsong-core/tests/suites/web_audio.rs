#![cfg(target_arch = "wasm32")]

use seedsong_core::{
    MusicBackend,
    backend::web_audio::WebAudioBackend,
    dna::Dna,
    music::{Event, Song},
    synth::{Options, Timbre},
};
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
async fn web_audio_renders_stereo_and_controls_source() {
    let mut backend = WebAudioBackend::new().unwrap();
    let mut song = Song::generate(Dna([0; 32]), [None; 4]);
    // A short phrase tests both synthesis and the real WebAudio API cheaply.
    song.events = vec![Event {
        start: 0,
        duration: 1,
        part: 0,
        pitch: 60,
        velocity: 100,
    }];
    for timbre in [Timbre::Spec, Timbre::Piano] {
        let buffer = backend
            .render(
                &song,
                Options {
                    rate: 22050,
                    timbre,
                    ..Options::default()
                },
            )
            .unwrap();
        assert_eq!(buffer.number_of_channels(), 2);
        assert_eq!(buffer.sample_rate(), 22050.);
        let left = buffer.get_channel_data(0).unwrap();
        let right = buffer.get_channel_data(1).unwrap();
        assert!(!left.is_empty());
        assert_eq!(left.len(), right.len());
        assert!(left.iter().chain(&right).all(|x| x.is_finite()));
        let peak = left
            .iter()
            .chain(&right)
            .map(|x| x.abs())
            .fold(0f32, f32::max);
        assert!((peak - 0.95).abs() < 1e-5);
        assert_ne!(left, right);
        // A suspended context can schedule sources without an autoplay gesture.
        let source = backend.play(&buffer, true).unwrap();
        assert!(source.loop_());
        assert!(source.buffer().is_some());
        let scheduled: &web_sys::AudioScheduledSourceNode = source.as_ref();
        scheduled.stop().unwrap();
        source.disconnect().unwrap();
    }
    JsFuture::from(backend.close().unwrap()).await.unwrap();
}

#[wasm_bindgen_test]
async fn web_audio_rejects_bad_input_before_rendering() {
    let mut backend = WebAudioBackend::new().unwrap();
    assert!(backend.render_dna("invalid", false, false).is_err());
    let song = Song::generate(Dna([0; 32]), [None; 4]);
    assert!(
        backend
            .render(
                &song,
                Options {
                    rate: 0,
                    ..Options::default()
                }
            )
            .is_err()
    );
    JsFuture::from(backend.close().unwrap()).await.unwrap();
}
