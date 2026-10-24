#![cfg(not(target_arch = "wasm32"))]
use seedsong_core::{
    backend::native,
    dna::Dna,
    music::{Event, Params, Song},
    synth::{self, Options, Timbre},
};
fn tiny_song() -> Song {
    let d = Dna([0; 32]);
    Song {
        version: Default::default(),
        params: Params::new(&d),
        sources: [d; 4],
        events: vec![
            Event {
                start: 0,
                duration: 1,
                part: 0,
                pitch: 60,
                velocity: 100,
            },
            Event {
                start: 0,
                duration: 1,
                part: 3,
                pitch: 38,
                velocity: 90,
            },
        ],
    }
}
#[test]
fn finite_normalized_and_independent_of_thread_count() {
    let song = tiny_song();
    for timbre in [Timbre::Spec, Timbre::Piano] {
        let render = || {
            synth::render(
                &song,
                Options {
                    rate: 22050,
                    timbre,
                    looping: false,
                    reverb: 0.18,
                },
            )
        };
        let a = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap()
            .install(render);
        let b = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap()
            .install(render);
        assert_eq!(a, b);
        let peak = a
            .iter()
            .flat_map(|x| x.iter())
            .map(|v| v.abs())
            .fold(0f32, f32::max);
        assert!(a.iter().flatten().all(|v| v.is_finite()));
        assert!((peak - 0.95).abs() < 1e-5);
        assert!(a.iter().any(|s| s[0] != s[1]));
        let path = std::env::temp_dir().join(format!(
            "seedsong-test-{}-{timbre:?}.wav",
            std::process::id()
        ));
        native::write_wav(&path, &a, 22050).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(bytes.len(), 44 + a.len() * 4);
        assert!(native::write_wav(&path, &a, 22050).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
#[test]
fn loop_has_exactly_sixteen_bars_and_keeps_wrapped_tails() {
    let mut song = tiny_song();
    song.events.truncate(1);
    song.events[0].start = 255;
    let a = synth::render(
        &song,
        Options {
            rate: 22050,
            timbre: Timbre::Piano,
            looping: true,
            reverb: 0.0,
        },
    );
    assert_eq!(a.len(), 256 * 22050 / 4);
    assert!(a[..22050 / 2].iter().any(|s| s[0].abs() > 0.01));
}
