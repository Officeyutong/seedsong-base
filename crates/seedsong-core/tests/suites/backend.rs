use seedsong_core::{
    MusicBackend, backend::PcmAudio, dna::Dna, generate, music::Song, synth::Options,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn generic_backend_receives_decoded_song_and_options() {
    struct EventBackend;
    impl MusicBackend for EventBackend {
        type Output = Vec<u8>;
        type Error = std::convert::Infallible;
        fn render(&mut self, song: &Song, options: Options) -> Result<Self::Output, Self::Error> {
            assert_eq!(options.rate, 48000);
            Ok(song.encode())
        }
    }
    let dna = Dna([0x2a; 32]);
    let actual = generate(
        &mut EventBackend,
        dna,
        [None; 4],
        Options {
            rate: 48000,
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(actual, Song::generate(dna, [None; 4]).encode());
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn wav_encoding_needs_no_filesystem() {
    let audio = PcmAudio {
        frames: vec![[0., 0.], [1., -1.]],
        rate: 44100,
    };
    let bytes = audio.to_wav().unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(bytes.len(), 52);
    assert_eq!(&bytes[44..], &[0, 0, 0, 0, 0xff, 0x7f, 1, 0x80]);
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
#[cfg_attr(not(target_arch = "wasm32"), test)]
fn invalid_render_options_are_rejected() {
    for rate in [0, 22049, 96001, u32::MAX] {
        assert!(
            Options {
                rate,
                ..Options::default()
            }
            .validate()
            .is_err()
        );
    }
    for reverb in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
        assert!(
            Options {
                reverb,
                ..Options::default()
            }
            .validate()
            .is_err()
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    assert!(
        generate(
            &mut seedsong_core::backend::native::NativeBackend,
            Dna([0; 32]),
            [None; 4],
            Options {
                rate: 0,
                ..Options::default()
            }
        )
        .is_err()
    );
}
